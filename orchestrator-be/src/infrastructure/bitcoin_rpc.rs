//! Bitcoin RPC client for orchestrator health checks only.
//!
//! On-chain broadcast executes in the desktop app (`desktop-app/src-tauri`).

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::error::AppError;
use crate::infrastructure::{http_client, rpc_timeout};

#[async_trait]
pub(crate) trait BitcoinRpcClient: Send + Sync {
    async fn estimate_fee_rate_sats_per_vb(&self, target_blocks: u16) -> Result<u64, AppError>;

    /// Return the block height at which `txid` was confirmed.
    ///
    /// Fails if the transaction is not yet in a block.
    async fn get_block_height_for_txid(&self, txid: &str) -> Result<u64, AppError>;

    /// Where a confirmed `txid` sits, with the transaction itself when this client reads it.
    ///
    /// Fails the same way as [`get_block_height_for_txid`] when the transaction is not in a block.
    /// The default reads no transaction; it exists for test doubles. The node client always
    /// returns it, so the reconcile can check that a mined reveal carries the proposal (#605).
    async fn get_mined_reveal(&self, txid: &str) -> Result<MinedReveal, AppError> {
        Ok(MinedReveal {
            height: self.get_block_height_for_txid(txid).await?,
            tx: None,
            block: None,
        })
    }

    /// The SPS-50 magic bytes of the network this node serves: the ASM processes an admin
    /// transaction only when its first output carries the tag under these bytes (#605). A test
    /// double answers the default.
    fn magic_bytes(&self) -> [u8; 4] {
        DEFAULT_MAGIC_BYTES
    }

    /// Return the current chain tip height (`getblockcount`).
    async fn get_chain_tip(&self) -> Result<u64, AppError>;

    /// Return where a mined `txid` sits in its block's transaction list (0 = coinbase). Used to
    /// order two reveals mined in the same block (#605). Fails if it is not in a block.
    async fn get_tx_position_in_block(&self, _txid: &str) -> Result<usize, AppError> {
        Err(AppError::BadRequest(
            "this bitcoin client cannot place a transaction in its block".to_string(),
        ))
    }
}

/// A transaction the node found in a block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MinedReveal {
    pub height: u64,
    /// The decoded transaction. `None` only from a client that does not read transactions.
    pub tx: Option<bitcoin::Transaction>,
    /// The hashes of its block and of that block's parent, which name the ASM states before and
    /// after the block. `None` only from a client that does not read them.
    pub block: Option<MinedBlock>,
}

/// Where a mined transaction sits: its block and the block before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MinedBlock {
    pub hash: String,
    pub parent_hash: String,
}

/// The node's transaction index, as `getindexinfo` reports it (#605).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TxIndex {
    /// The node runs without `-txindex`.
    Missing,
    /// The index exists but has not caught up with the chain yet.
    Syncing,
    Synced,
}

pub(crate) struct HttpBitcoinRpcClient {
    url: String,
    user: String,
    pass: String,
    client: reqwest::Client,
    magic_bytes: [u8; 4],
}

/// The magic bytes when none are configured: the same default as the desktop (`ALPN`).
pub(crate) const DEFAULT_MAGIC_BYTES: [u8; 4] = *b"ALPN";

impl HttpBitcoinRpcClient {
    pub(crate) fn new(base_url: &str, user: &str, pass: &str) -> Self {
        Self {
            url: base_url.trim_end_matches('/').to_string(),
            user: user.to_string(),
            pass: pass.to_string(),
            client: http_client::shared(),
            magic_bytes: DEFAULT_MAGIC_BYTES,
        }
    }

    /// The network's SPS-50 magic bytes, as configured for the ASM and the desktop.
    pub(crate) fn with_magic_bytes(mut self, magic_bytes: [u8; 4]) -> Self {
        self.magic_bytes = magic_bytes;
        self
    }

    /// Whether the node keeps a transaction index (#605). Without one, `getrawtransaction` cannot
    /// find a mined transaction, so the reconcile cannot tell which of several pairs was mined.
    /// `Err` when the node could not be asked.
    pub(crate) async fn txindex(&self) -> Result<TxIndex, AppError> {
        let info = self.call("getindexinfo", json!(["txindex"])).await?;
        Ok(match info.get("txindex") {
            None => TxIndex::Missing,
            Some(index) if index.get("synced").and_then(Value::as_bool) == Some(true) => {
                TxIndex::Synced
            }
            Some(_) => TxIndex::Syncing,
        })
    }

    /// `getrawtransaction` for `txid` and the height of the block holding it.
    async fn locate(&self, txid: &str) -> Result<(Value, u64), AppError> {
        // getrawtransaction works for any tx via txindex, unlike gettransaction which
        // requires the tx to be in the node's wallet.
        let result = self.call("getrawtransaction", json!([txid, true])).await?;

        // Bitcoin Core 23+ includes blockheight directly; fall back to getblockheader.
        if let Some(height) = result.get("blockheight").and_then(|v| v.as_u64()) {
            return Ok((result, height));
        }

        let blockhash = result
            .get("blockhash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::BadRequest(format!(
                    "getrawtransaction: tx {txid} has no blockhash — not yet confirmed"
                ))
            })?;

        let header = self
            .call("getblockheader", json!([blockhash, true]))
            .await?;

        let height = header
            .get("height")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                AppError::BadRequest(format!(
                    "getblockheader: missing `height` for blockhash {blockhash}"
                ))
            })?;
        Ok((result, height))
    }

    /// Any RPC call, for tests that drive a real regtest node.
    #[cfg(test)]
    pub(crate) async fn rpc(&self, method: &str, params: Value) -> Result<Value, AppError> {
        self.call(method, params).await
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
        .map_err(|e| AppError::BadRequest(format!("bitcoin rpc `{method}` request failed: {e}")))?;

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
            return Err(AppError::BadRequest(format!(
                "bitcoin rpc `{method}` failed (HTTP {status}): {msg}"
            )));
        }

        let body: Value = resp.json().await.map_err(|e| {
            AppError::BadRequest(format!("bitcoin rpc `{method}` invalid json: {e}"))
        })?;

        if let Some(err) = body.get("error").filter(|v| !v.is_null()) {
            let msg = err
                .pointer("/message")
                .and_then(|v| v.as_str())
                .unwrap_or(&err.to_string())
                .to_string();
            return Err(AppError::BadRequest(format!(
                "bitcoin rpc `{method}` error: {msg}"
            )));
        }

        body.get("result")
            .cloned()
            .ok_or_else(|| AppError::BadRequest(format!("bitcoin rpc `{method}` missing result")))
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
        Ok(self.locate(txid).await?.1)
    }

    async fn get_mined_reveal(&self, txid: &str) -> Result<MinedReveal, AppError> {
        let result = self.call("getrawtransaction", json!([txid, true])).await?;
        let blockhash = result
            .get("blockhash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::BadRequest(format!(
                    "getrawtransaction: tx {txid} has no blockhash — not yet confirmed"
                ))
            })?
            .to_string();
        let header = self
            .call("getblockheader", json!([blockhash, true]))
            .await?;
        let height = header
            .get("height")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                AppError::BadRequest(format!(
                    "getblockheader: missing `height` for blockhash {blockhash}"
                ))
            })?;
        let parent_hash = header
            .get("previousblockhash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::BadRequest(format!(
                    "getblockheader: missing `previousblockhash` for blockhash {blockhash}"
                ))
            })?
            .to_string();
        let tx = result
            .get("hex")
            .and_then(Value::as_str)
            .and_then(|hex| bitcoin::consensus::encode::deserialize_hex(hex).ok())
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!(
                    "getrawtransaction: tx {txid} came back without a decodable `hex`"
                ))
            })?;
        Ok(MinedReveal {
            height,
            tx: Some(tx),
            block: Some(MinedBlock {
                hash: blockhash,
                parent_hash,
            }),
        })
    }

    async fn get_tx_position_in_block(&self, txid: &str) -> Result<usize, AppError> {
        let tx = self.call("getrawtransaction", json!([txid, true])).await?;
        let blockhash = tx
            .get("blockhash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AppError::BadRequest(format!(
                    "getrawtransaction: tx {txid} has no blockhash — not yet confirmed"
                ))
            })?;
        let block = self.call("getblock", json!([blockhash, 1])).await?;
        block
            .get("tx")
            .and_then(|v| v.as_array())
            .and_then(|txids| {
                txids
                    .iter()
                    .position(|t| t.as_str().is_some_and(|t| t.eq_ignore_ascii_case(txid)))
            })
            .ok_or_else(|| {
                AppError::BadRequest(format!("getblock: tx {txid} is not in block {blockhash}"))
            })
    }

    fn magic_bytes(&self) -> [u8; 4] {
        self.magic_bytes
    }

    async fn get_chain_tip(&self) -> Result<u64, AppError> {
        let result = self.call("getblockcount", json!([])).await?;
        result.as_u64().ok_or_else(|| {
            AppError::BadRequest("bitcoin rpc `getblockcount` did not return a u64".to_string())
        })
    }
}
