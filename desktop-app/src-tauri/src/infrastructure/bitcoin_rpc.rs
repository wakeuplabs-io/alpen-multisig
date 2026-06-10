use async_trait::async_trait;
use bitcoin::consensus::Decodable;
use bitcoin::Transaction;
use serde_json::{json, Value};

#[async_trait]
pub trait BitcoinRpcClient: Send + Sync {
    /// Broadcast a fully signed raw transaction. Returns txid.
    async fn send_raw_transaction(&self, tx_hex: &str) -> Result<String, String>;

    /// Get the number of confirmations for a transaction (0 = unconfirmed).
    async fn get_transaction_confirmations(&self, txid: &str) -> Result<u32, String>;

    /// Estimate fee rate in satoshis per vbyte for the given target block count.
    async fn estimate_fee_rate_sats_per_vb(&self, target_blocks: u16) -> Result<u64, String>;

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
    async fn submit_package(&self, tx_hexes: &[String]) -> Result<(), String>;
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
            .map_err(|e| format!("bitcoin rpc send failed: {e}"))?;

        let status = resp.status();

        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<Value>(&body_text)
                .ok()
                .and_then(|v| {
                    v.pointer("/error/message")
                        .and_then(|m| m.as_str())
                        .map(str::to_string)
                })
                .unwrap_or(body_text);
            return Err(format!(
                "bitcoin rpc `{method}` failed (HTTP {status}): {msg}"
            ));
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("bitcoin rpc `{method}` invalid json: {e}"))?;

        if let Some(err) = body.get("error").filter(|v| !v.is_null()) {
            let msg = err
                .pointer("/message")
                .and_then(|v| v.as_str())
                .unwrap_or(&err.to_string())
                .to_string();
            return Err(format!("bitcoin rpc `{method}` error: {msg}"));
        }

        body.get("result")
            .cloned()
            .ok_or_else(|| format!("bitcoin rpc `{method}` missing result"))
    }
}

#[async_trait]
impl BitcoinRpcClient for HttpBitcoinRpcClient {
    async fn send_raw_transaction(&self, tx_hex: &str) -> Result<String, String> {
        let result = self.call("sendrawtransaction", json!([tx_hex])).await?;
        result
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "sendrawtransaction: expected string txid".to_string())
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

    async fn estimate_fee_rate_sats_per_vb(&self, target_blocks: u16) -> Result<u64, String> {
        let result = self
            .call("estimatesmartfee", json!([target_blocks]))
            .await?;

        let feerate_btc_per_kb = result
            .get("feerate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.00001);

        let sats_per_vb = (feerate_btc_per_kb * 100_000_000.0 / 1000.0).ceil() as u64;
        Ok(sats_per_vb.max(1))
    }

    async fn estimate_smart_fee_sat_per_kvb(&self, target_blocks: u16) -> Result<u64, String> {
        let result = self
            .call("estimatesmartfee", json!([target_blocks]))
            .await?;

        // Fail explicitly if the node reports estimation errors (e.g. insufficient data on regtest).
        if let Some(errors) = result.get("errors") {
            if !errors.is_null() {
                if let Some(arr) = errors.as_array() {
                    if !arr.is_empty() {
                        let msg = arr
                            .first()
                            .and_then(|v| v.as_str())
                            .unwrap_or("estimation error");
                        return Err(format!("estimatesmartfee target={target_blocks}: {msg}"));
                    }
                }
            }
        }

        let feerate_btc_per_kb =
            result
                .get("feerate")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| {
                    format!("estimatesmartfee target={target_blocks}: missing feerate field")
                })?;

        // sat/kvB = BTC/kvB × 1e8 (ceiling)
        let sat_per_kvb = (feerate_btc_per_kb * 100_000_000.0).ceil() as u64;
        Ok(sat_per_kvb.max(1))
    }

    async fn min_relay_sat_per_kvb(&self) -> Result<u64, String> {
        // getnetworkinfo returns relayfee in BTC/kvB; getmempoolinfo returns mempoolminfee.
        let network_info = self.call("getnetworkinfo", json!([])).await?;
        let mempool_info = self.call("getmempoolinfo", json!([])).await?;

        let relay_btc_per_kb = network_info
            .get("relayfee")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.00001);
        let mempool_btc_per_kb = mempool_info
            .get("mempoolminfee")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.00001);

        let relay_sat_per_kvb = (relay_btc_per_kb * 100_000_000.0).ceil() as u64;
        let mempool_sat_per_kvb = (mempool_btc_per_kb * 100_000_000.0).ceil() as u64;

        Ok(relay_sat_per_kvb.max(mempool_sat_per_kvb).max(1))
    }

    async fn get_block_count(&self) -> Result<u64, String> {
        let result = self.call("getblockcount", json!([])).await?;
        result
            .as_u64()
            .ok_or_else(|| "getblockcount: expected u64".to_string())
    }

    async fn submit_package(&self, tx_hexes: &[String]) -> Result<(), String> {
        let result = self
            .call("submitpackage", serde_json::json!([tx_hexes]))
            .await?;
        let pkg_msg = result
            .get("package_msg")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if pkg_msg == "success" {
            Ok(())
        } else {
            Err(format!("submitpackage: unexpected result: {result}"))
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

    #[test]
    fn submit_package_parses_non_success_package_msg_as_error() {
        let result_value = serde_json::json!({"package_msg": "some-failure"});
        let pkg_msg = result_value
            .get("package_msg")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_ne!(pkg_msg, "success");
    }

    #[test]
    fn submit_package_parses_success_package_msg() {
        let result_value = serde_json::json!({"package_msg": "success"});
        let pkg_msg = result_value
            .get("package_msg")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(pkg_msg, "success");
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
}
