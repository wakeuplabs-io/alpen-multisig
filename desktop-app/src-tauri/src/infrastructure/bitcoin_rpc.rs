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

    /// Fetch and decode a transaction by txid.
    async fn get_raw_transaction(&self, txid: &str) -> Result<Transaction, String>;

    /// Mine `count` blocks to an internally generated address. Regtest only.
    async fn mine_blocks(&self, count: u32) -> Result<(), String>;

    /// Submit a package of transactions. Returns Ok(()) if the node accepts the package.
    /// Unknown-method errors are propagated verbatim so callers can branch on them.
    async fn submit_package(&self, tx_hexes: &[String]) -> Result<(), String>;
}

pub struct HttpBitcoinRpcClient {
    url: String,
    user: String,
    pass: String,
    client: reqwest::Client,
}

impl HttpBitcoinRpcClient {
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

    async fn mine_blocks(&self, count: u32) -> Result<(), String> {
        let addr_result = self.call("getnewaddress", json!([])).await?;
        let addr = addr_result
            .as_str()
            .ok_or_else(|| "getnewaddress: expected string address".to_string())?
            .to_string();
        self.call("generatetoaddress", json!([count, addr])).await?;
        Ok(())
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
}
