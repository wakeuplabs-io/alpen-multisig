//! Bitcoin RPC client for orchestrator health checks only.
//!
//! On-chain broadcast executes in the desktop app (`desktop-app/src-tauri`).

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::error::AppError;
use crate::infrastructure::rpc_timeout;

#[async_trait]
pub(crate) trait BitcoinRpcClient: Send + Sync {
    async fn estimate_fee_rate_sats_per_vb(&self, target_blocks: u16) -> Result<u64, AppError>;

    /// Return the block height at which `txid` was confirmed.
    ///
    /// Fails if the transaction is not yet in a block.
    async fn get_block_height_for_txid(&self, txid: &str) -> Result<u64, AppError>;
}

pub(crate) struct HttpBitcoinRpcClient {
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

        let resp = rpc_timeout::with_rpc_timeout(
            "Bitcoin RPC",
            self.client
                .post(&self.url)
                .basic_auth(&self.user, Some(&self.pass))
                .json(&payload)
                .send(),
        )
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?;

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

    async fn get_block_height_for_txid(&self, txid: &str) -> Result<u64, AppError> {
        let result = self
            .call("gettransaction", json!([txid, true]))
            .await?;

        result
            .get("blockheight")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "gettransaction: missing `blockheight` for txid {txid} — not yet confirmed?"
                ))
            })
    }
}
