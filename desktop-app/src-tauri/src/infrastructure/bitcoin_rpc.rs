use async_trait::async_trait;
use bitcoin::consensus::Decodable;
use bitcoin::Transaction;
use serde_json::{json, Value};

#[async_trait]
pub trait BitcoinRpcClient: Send + Sync {
    /// Broadcast a fully signed raw transaction. Returns txid. The error says whether the node
    /// answered, so a broadcaster can tell a rejection from a request that may have landed.
    async fn send_raw_transaction(&self, tx_hex: &str) -> Result<String, RpcError>;

    /// Get the number of confirmations for a transaction (0 = unconfirmed).
    async fn get_transaction_confirmations(&self, txid: &str) -> Result<u32, String>;

    /// Strict fee estimate in sat/kvB via `estimatesmartfee`.
    ///
    /// Returns `Err` if the node reports `errors`, if the `feerate` field is missing,
    /// or if the RPC call itself fails. Never swallows errors into a default value.
    async fn estimate_smart_fee_sat_per_kvb(&self, target_blocks: u16) -> Result<u64, String>;

    /// Minimum relay fee in sat/kvB: `max(getnetworkinfo.relayfee, getmempoolinfo.mempoolminfee)`.
    async fn min_relay_sat_per_kvb(&self) -> Result<u64, String>;

    /// Fetch and decode a transaction by txid.
    async fn get_raw_transaction(&self, txid: &str) -> Result<Transaction, String>;

    /// Get the current block count.
    async fn get_block_count(&self) -> Result<u64, String>;

    /// Submit a package of transactions.
    async fn submit_package(&self, tx_hexes: &[String]) -> Result<(), RpcError>;
}

/// How a failed node RPC call ended, for callers that must know whether the request may have
/// taken effect (#516).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcFailureKind {
    /// The node answered with a JSON-RPC error object.
    Answered,
    /// The node cannot have run the call: the connection never opened, or the HTTP server
    /// refused the request before dispatching it (401/403/404/405 without a JSON-RPC error).
    NotConnected,
    /// Anything else — a timeout, a connection dropped mid-request, any other HTTP error without
    /// a JSON-RPC error body (5xx), an unreadable or unexpected result. The node may have acted
    /// on it.
    Unknown,
}

/// A failed node RPC call.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct RpcError {
    pub kind: RpcFailureKind,
    /// The JSON-RPC error code, when the node answered with one.
    pub code: Option<i64>,
    pub message: String,
}

impl RpcError {
    pub fn answered(code: Option<i64>, message: impl Into<String>) -> Self {
        Self {
            kind: RpcFailureKind::Answered,
            code,
            message: message.into(),
        }
    }

    pub fn not_connected(message: impl Into<String>) -> Self {
        Self {
            kind: RpcFailureKind::NotConnected,
            code: None,
            message: message.into(),
        }
    }

    pub fn unknown(message: impl Into<String>) -> Self {
        Self {
            kind: RpcFailureKind::Unknown,
            code: None,
            message: message.into(),
        }
    }
}

pub struct HttpBitcoinRpcClient {
    url: String,
    user: String,
    pass: String,
    client: reqwest::Client,
}

impl HttpBitcoinRpcClient {
    /// Build a node-level Bitcoin RPC client. The broadcast path only uses node/mempool
    /// RPCs (sendrawtransaction, submitpackage, estimatesmartfee, getrawtransaction,
    /// gettransaction, getblockcount), so the client is NEVER wallet-scoped. Any
    /// bitcoind Core-wallet operation (funding, addresses, mining) is out of scope —
    /// funding/signing goes through the BDK Admin Wallet, mining through the dev faucet.
    pub fn new(base_url: &str, user: &str, pass: &str) -> Self {
        Self {
            url: base_url.to_string(),
            user: user.to_string(),
            pass: pass.to_string(),
            client: super::rpc_timeout::rpc_client(),
        }
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value, String> {
        self.call_typed(method, params).await.map_err(|e| e.message)
    }

    /// Like [`Self::call`], keeping how the call failed (#516).
    async fn call_typed(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let resp = self
            .client
            .post(&self.url)
            .basic_auth(&self.user, Some(&self.pass))
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                let message = format!("bitcoin rpc send failed: {e}");
                if e.is_connect() {
                    RpcError::not_connected(message)
                } else {
                    RpcError::unknown(message)
                }
            })?;

        let status = resp.status();
        // Bitcoin Core answers an RPC error with a JSON body carrying `error`, often under an
        // HTTP error status; that body is what makes the failure an answer.
        let body_text = resp
            .text()
            .await
            .map_err(|e| RpcError::unknown(format!("bitcoin rpc `{method}` unreadable: {e}")))?;
        let body = serde_json::from_str::<Value>(&body_text).ok();

        if let Some(err) = body
            .as_ref()
            .and_then(|b| b.get("error"))
            .filter(|v| !v.is_null())
        {
            let msg = err
                .get("message")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| err.to_string());
            let code = err.get("code").and_then(Value::as_i64);
            let message = if status.is_success() {
                format!("bitcoin rpc `{method}` error: {msg}")
            } else {
                format!("bitcoin rpc `{method}` failed (HTTP {status}): {msg}")
            };
            return Err(RpcError::answered(code, message));
        }

        if !status.is_success() {
            let message = format!("bitcoin rpc `{method}` failed (HTTP {status}): {body_text}");
            return Err(if refused_before_dispatch(status) {
                RpcError::not_connected(message)
            } else {
                RpcError::unknown(message)
            });
        }

        body.ok_or_else(|| RpcError::unknown(format!("bitcoin rpc `{method}` invalid json")))?
            .get("result")
            .cloned()
            .ok_or_else(|| RpcError::unknown(format!("bitcoin rpc `{method}` missing result")))
    }
}

/// Bitcoin Core's HTTP server answers these statuses without a JSON-RPC body before any RPC runs
/// — bad credentials (401), a forbidden client (403), an unknown path (404), a non-POST request
/// (405) — so the call cannot have taken effect (#516). A 5xx without a body is not one of them:
/// it may come after the RPC ran.
fn refused_before_dispatch(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 401 | 403 | 404 | 405)
}

/// BTC/kvB (float from RPC) → sat/kvB (integer, never below 1).
///
/// BTC amounts have exactly 8 decimals, so the true value is always a whole
/// number of sats; `round()` recovers it exactly. (`ceil()` would inflate
/// 0.00001 BTC to 1_001 sat due to IEEE-754 noise: 0.00001 × 1e8 = 1000.0000000000001.)
fn btc_per_kvb_to_sat_per_kvb(btc_per_kvb: f64) -> u64 {
    ((btc_per_kvb * 100_000_000.0).round() as u64).max(1)
}

/// Parse an `estimatesmartfee` result strictly: any reported error or a missing
/// `feerate` field is an `Err` — never swallowed into a default value.
fn parse_estimate_smart_fee(result: &Value, target_blocks: u16) -> Result<u64, String> {
    let errors = result.get("errors").and_then(|v| v.as_array());
    if let Some(msg) = errors.and_then(|arr| arr.first()) {
        let msg = msg.as_str().unwrap_or("estimation error");
        return Err(format!("estimatesmartfee target={target_blocks}: {msg}"));
    }

    let feerate_btc_per_kvb = result
        .get("feerate")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| format!("estimatesmartfee target={target_blocks}: missing feerate field"))?;

    Ok(btc_per_kvb_to_sat_per_kvb(feerate_btc_per_kvb))
}

/// Effective minimum relay fee in sat/kvB: `max(relayfee, mempoolminfee)`.
/// Missing fields default to 1 sat/vB (0.00001 BTC/kvB) — a safe floor, not a fee.
fn parse_min_relay(network_info: &Value, mempool_info: &Value) -> u64 {
    let relay = network_info
        .get("relayfee")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.00001);
    let mempool = mempool_info
        .get("mempoolminfee")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.00001);
    btc_per_kvb_to_sat_per_kvb(relay).max(btc_per_kvb_to_sat_per_kvb(mempool))
}

#[async_trait]
impl BitcoinRpcClient for HttpBitcoinRpcClient {
    async fn send_raw_transaction(&self, tx_hex: &str) -> Result<String, RpcError> {
        let result = self
            .call_typed("sendrawtransaction", json!([tx_hex]))
            .await?;
        result
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| RpcError::unknown("sendrawtransaction: expected string txid"))
    }

    async fn get_transaction_confirmations(&self, txid: &str) -> Result<u32, String> {
        // Try wallet RPC first; fall back to getrawtransaction for non-wallet txs.
        let result = match self
            .call("gettransaction", json!([txid, false, false]))
            .await
        {
            Ok(v) => v,
            Err(_) => {
                let raw = self.call("getrawtransaction", json!([txid, true])).await?;
                // Unconfirmed mempool txs have no `confirmations` field — treat as 0.
                let confs = raw
                    .get("confirmations")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                return Ok(confs.max(0) as u32);
            }
        };
        let confs = result
            .get("confirmations")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        Ok(confs.max(0) as u32)
    }

    async fn estimate_smart_fee_sat_per_kvb(&self, target_blocks: u16) -> Result<u64, String> {
        let result = self
            .call("estimatesmartfee", json!([target_blocks]))
            .await?;
        parse_estimate_smart_fee(&result, target_blocks)
    }

    async fn min_relay_sat_per_kvb(&self) -> Result<u64, String> {
        // getnetworkinfo returns relayfee in BTC/kvB; getmempoolinfo returns mempoolminfee.
        let network_info = self.call("getnetworkinfo", json!([])).await?;
        let mempool_info = self.call("getmempoolinfo", json!([])).await?;
        Ok(parse_min_relay(&network_info, &mempool_info))
    }

    async fn get_block_count(&self) -> Result<u64, String> {
        let result = self.call("getblockcount", json!([])).await?;
        result
            .as_u64()
            .ok_or_else(|| "getblockcount: expected u64".to_string())
    }

    async fn submit_package(&self, tx_hexes: &[String]) -> Result<(), RpcError> {
        let result = self
            .call_typed("submitpackage", serde_json::json!([tx_hexes]))
            .await?;
        let pkg_msg = result
            .get("package_msg")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if pkg_msg == "success" {
            Ok(())
        } else {
            // The node answered without taking the whole package (#516): part of it may be in
            // the mempool, so the caller asks again tx by tx.
            Err(RpcError::answered(
                None,
                format!("submitpackage: package not accepted: {result}"),
            ))
        }
    }

    async fn get_raw_transaction(&self, txid: &str) -> Result<Transaction, String> {
        let result = self.call("getrawtransaction", json!([txid, false])).await?;
        let hex_str = result
            .as_str()
            .ok_or_else(|| "getrawtransaction: expected hex string".to_string())?;

        let tx_bytes =
            hex::decode(hex_str).map_err(|e| format!("getrawtransaction: invalid hex: {e}"))?;

        Transaction::consensus_decode(&mut tx_bytes.as_slice())
            .map_err(|e| format!("getrawtransaction: decode failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::rpc_timeout::{rpc_client, RPC_TIMEOUT};
    use super::BitcoinRpcClient;

    #[test]
    fn submit_package_is_on_bitcoin_rpc_client_trait() {
        // compile-gate: submit_package must be on BitcoinRpcClient
        fn _accepts_trait_object(_: &dyn BitcoinRpcClient) {}
    }

    // ─── how a failed call ended (#516) ───────────────────────────────────────

    use super::{RpcError, RpcFailureKind};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A node stub on a local port that reads one request and answers `response` (raw HTTP),
    /// or closes the connection without answering when `None`.
    async fn one_shot_node(response: Option<String>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 16 * 1024];
            let _ = socket.read(&mut buf).await;
            if let Some(response) = response {
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });
        format!("http://{addr}")
    }

    fn http(status: &str, body: &str) -> Option<String> {
        Some(format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ))
    }

    /// A port nothing listens on: the connection is refused.
    async fn closed_port() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        format!("http://{addr}")
    }

    /// #516: only a JSON-RPC error answer is a node's answer. A connection that never opened, or
    /// an HTTP refusal Core sends before dispatching any RPC (401/403/404/405), proves nothing
    /// ran. Everything else may have reached the node.
    #[tokio::test]
    async fn broadcast_failures_are_classified_by_how_the_call_ended() {
        let rejection =
            r#"{"result":null,"error":{"code":-26,"message":"min relay fee not met"},"id":1}"#;
        type Expected = Result<String, (RpcFailureKind, Option<i64>)>;
        let cases: Vec<(String, Expected)> = vec![
            (
                one_shot_node(http("200 OK", r#"{"result":"ab12","error":null,"id":1}"#)).await,
                Ok("ab12".to_string()),
            ),
            (
                one_shot_node(http("500 Internal Server Error", rejection)).await,
                Err((RpcFailureKind::Answered, Some(-26))),
            ),
            (
                one_shot_node(http("401 Unauthorized", "")).await,
                Err((RpcFailureKind::NotConnected, None)),
            ),
            (
                one_shot_node(http("403 Forbidden", "")).await,
                Err((RpcFailureKind::NotConnected, None)),
            ),
            (
                one_shot_node(http("404 Not Found", "")).await,
                Err((RpcFailureKind::NotConnected, None)),
            ),
            (
                one_shot_node(http("405 Method Not Allowed", "")).await,
                Err((RpcFailureKind::NotConnected, None)),
            ),
            (
                one_shot_node(http("500 Internal Server Error", "")).await,
                Err((RpcFailureKind::Unknown, None)),
            ),
            (
                one_shot_node(http("503 Service Unavailable", "busy")).await,
                Err((RpcFailureKind::Unknown, None)),
            ),
            (
                one_shot_node(None).await,
                Err((RpcFailureKind::Unknown, None)),
            ),
            (
                closed_port().await,
                Err((RpcFailureKind::NotConnected, None)),
            ),
        ];
        for (url, expected) in cases {
            let client = super::HttpBitcoinRpcClient::new(&url, "user", "pass");
            let result = client
                .send_raw_transaction("00")
                .await
                .map_err(|e: RpcError| (e.kind, e.code));
            assert_eq!(result, expected, "{url}");
        }
    }

    /// #516: a `submitpackage` result without `package_msg: success` is the node's answer — part
    /// of the package may be in the mempool — so it is `Answered`, never a transport failure.
    #[tokio::test]
    async fn submit_package_non_success_package_msg_is_an_answer() {
        let body = r#"{"result":{"package_msg":"transaction failed","tx-results":{}},"error":null,"id":1}"#;
        let url = one_shot_node(http("200 OK", body)).await;

        let e = super::HttpBitcoinRpcClient::new(&url, "user", "pass")
            .submit_package(&["00".to_string()])
            .await
            .unwrap_err();

        assert_eq!(e.kind, RpcFailureKind::Answered, "{}", e.message);
    }

    #[test]
    fn rpc_timeout_is_thirty_seconds() {
        assert_eq!(RPC_TIMEOUT.as_secs(), 30);
    }

    #[test]
    fn rpc_client_builds_without_panic() {
        let _client = rpc_client();
    }

    /// Regression (RCA: getnewaddress "wallet does not exist or is not loaded").
    /// The production RPC client must be node-level — never wallet-scoped — so the
    /// broadcast path never depends on a bitcoind Core wallet being loaded.
    #[test]
    fn http_client_is_node_level_not_wallet_scoped() {
        let client = super::HttpBitcoinRpcClient::new("http://127.0.0.1:18443", "user", "pass");
        assert_eq!(client.url, "http://127.0.0.1:18443");
        assert!(
            !client.url.contains("/wallet/"),
            "production RPC client must not be wallet-scoped"
        );
    }

    // ─── estimatesmartfee / min-relay parsing ────────────────────────────────

    use super::{btc_per_kvb_to_sat_per_kvb, parse_estimate_smart_fee, parse_min_relay};
    use serde_json::json;

    #[test]
    fn estimate_smart_fee_parses_feerate_btc_to_sat_per_kvb() {
        // 0.00001 BTC/kvB = 1_000 sat/kvB (1 sat/vB)
        let result = json!({"feerate": 0.00001, "blocks": 6});
        assert_eq!(parse_estimate_smart_fee(&result, 6).unwrap(), 1_000);
    }

    #[test]
    fn estimate_smart_fee_recovers_exact_sat_value_despite_float_noise() {
        // Regression: 0.00001 × 1e8 = 1000.0000000000001 in IEEE-754; ceil() would
        // inflate it to 1_001. BTC has 8 decimals, so round() recovers the exact value.
        let result = json!({"feerate": 0.00001234, "blocks": 6});
        assert_eq!(parse_estimate_smart_fee(&result, 6).unwrap(), 1_234);
    }

    #[test]
    fn estimate_smart_fee_rejects_errors_array() {
        // Typical regtest response: no estimate available yet.
        let result = json!({"errors": ["Insufficient data or no feerate found"], "blocks": 6});
        let err = parse_estimate_smart_fee(&result, 6).unwrap_err();
        assert!(err.contains("Insufficient data"), "got: {err}");
        assert!(err.contains("target=6"), "got: {err}");
    }

    #[test]
    fn estimate_smart_fee_rejects_missing_feerate() {
        let result = json!({"blocks": 6});
        let err = parse_estimate_smart_fee(&result, 6).unwrap_err();
        assert!(err.contains("missing feerate"), "got: {err}");
    }

    #[test]
    fn estimate_smart_fee_empty_errors_array_with_feerate_succeeds() {
        let result = json!({"errors": [], "feerate": 0.00002, "blocks": 6});
        assert_eq!(parse_estimate_smart_fee(&result, 6).unwrap(), 2_000);
    }

    #[test]
    fn min_relay_takes_max_of_relayfee_and_mempoolminfee() {
        // mempoolminfee elevated above relayfee (mempool under pressure)
        let network = json!({"relayfee": 0.00001});
        let mempool = json!({"mempoolminfee": 0.00005});
        assert_eq!(parse_min_relay(&network, &mempool), 5_000);
    }

    #[test]
    fn min_relay_missing_fields_default_to_one_sat_per_vb() {
        assert_eq!(parse_min_relay(&json!({}), &json!({})), 1_000);
    }

    #[test]
    fn btc_to_sat_conversion_never_returns_zero() {
        assert_eq!(btc_per_kvb_to_sat_per_kvb(0.0), 1);
    }
}
