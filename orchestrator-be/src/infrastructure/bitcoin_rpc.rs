use async_trait::async_trait;
use bitcoin::consensus::Decodable;
use bitcoin::Transaction;
use serde_json::{json, Value};

use crate::error::AppError;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

#[async_trait]
pub(crate) trait BitcoinRpcClient: Send + Sync {
    /// Fund the taproot commit address from the node wallet. Returns txid.
    /// `fee_rate_sats_per_vb` is passed directly to avoid relying on the node's fee estimator.
    async fn send_to_address(
        &self,
        address: &str,
        amount_sats: u64,
        fee_rate_sats_per_vb: u64,
    ) -> Result<String, AppError>;

    /// Broadcast a fully signed raw transaction. Returns txid.
    async fn send_raw_transaction(&self, tx_hex: &str) -> Result<String, AppError>;

    /// Get the number of confirmations for a transaction (0 = unconfirmed).
    async fn get_transaction_confirmations(&self, txid: &str) -> Result<u32, AppError>;

    /// Estimate fee rate in satoshis per vbyte for the given target block count.
    async fn estimate_fee_rate_sats_per_vb(&self, target_blocks: u16) -> Result<u64, AppError>;

    /// Fetch and decode a transaction by txid.
    async fn get_raw_transaction(&self, txid: &str) -> Result<Transaction, AppError>;
}

// ---------------------------------------------------------------------------
// HTTP JSON-RPC implementation
// ---------------------------------------------------------------------------

pub(crate) struct HttpBitcoinRpcClient {
    /// Full RPC URL, optionally including wallet: http://host:port/wallet/<name>
    url: String,
    user: String,
    pass: String,
    client: reqwest::Client,
}

impl HttpBitcoinRpcClient {
    pub(crate) fn new(base_url: &str, wallet_name: Option<&str>, user: &str, pass: &str) -> Self {
        let url = match wallet_name.filter(|w| !w.is_empty()) {
            Some(wallet) => format!("{}/wallet/{}", base_url.trim_end_matches('/'), wallet),
            None => base_url.to_string(),
        };
        Self {
            url,
            user: user.to_string(),
            pass: pass.to_string(),
            client: reqwest::Client::new(),
        }
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value, AppError> {
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
            .map_err(|e| AppError::Internal(anyhow::anyhow!("bitcoin rpc send failed: {e}")))?;

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
            return Err(AppError::Internal(anyhow::anyhow!(
                "bitcoin rpc `{method}` failed (HTTP {status}): {msg}"
            )));
        }

        let body: Value = resp.json().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("bitcoin rpc `{method}` invalid json: {e}"))
        })?;

        if let Some(err) = body.get("error").filter(|v| !v.is_null()) {
            let msg = err
                .pointer("/message")
                .and_then(|v| v.as_str())
                .unwrap_or(&err.to_string())
                .to_string();
            return Err(AppError::Internal(anyhow::anyhow!(
                "bitcoin rpc `{method}` error: {msg}"
            )));
        }

        body.get("result").cloned().ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("bitcoin rpc `{method}` missing result"))
        })
    }
}

#[async_trait]
impl BitcoinRpcClient for HttpBitcoinRpcClient {
    async fn send_to_address(
        &self,
        address: &str,
        amount_sats: u64,
        fee_rate_sats_per_vb: u64,
    ) -> Result<String, AppError> {
        let btc_amount = amount_sats as f64 / 100_000_000.0;
        let result = self
            .call(
                "sendtoaddress",
                json!([
                    address,
                    btc_amount,
                    "",
                    "",
                    false,
                    null,
                    null,
                    "unset",
                    null,
                    fee_rate_sats_per_vb
                ]),
            )
            .await?;
        result.as_str().map(str::to_string).ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("sendtoaddress: expected string txid"))
        })
    }

    async fn send_raw_transaction(&self, tx_hex: &str) -> Result<String, AppError> {
        let result = self.call("sendrawtransaction", json!([tx_hex])).await?;
        result.as_str().map(str::to_string).ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("sendrawtransaction: expected string txid"))
        })
    }

    async fn get_transaction_confirmations(&self, txid: &str) -> Result<u32, AppError> {
        let result = self
            .call("gettransaction", json!([txid, false, false]))
            .await?;
        let confs = result
            .get("confirmations")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        Ok(confs.max(0) as u32)
    }

    async fn estimate_fee_rate_sats_per_vb(&self, target_blocks: u16) -> Result<u64, AppError> {
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

    async fn get_raw_transaction(&self, txid: &str) -> Result<Transaction, AppError> {
        let result = self.call("getrawtransaction", json!([txid, false])).await?;
        let hex_str = result.as_str().ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("getrawtransaction: expected hex string"))
        })?;

        let tx_bytes = hex::decode(hex_str).map_err(|e| {
            AppError::Internal(anyhow::anyhow!("getrawtransaction: invalid hex: {e}"))
        })?;

        Transaction::consensus_decode(&mut tx_bytes.as_slice()).map_err(|e| {
            AppError::Internal(anyhow::anyhow!("getrawtransaction: decode failed: {e}"))
        })
    }
}
