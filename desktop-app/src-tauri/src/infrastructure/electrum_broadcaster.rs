//! [`TxBroadcaster`] implementation over the Electrum protocol (M3).
//!
//! Uses `bdk_electrum::electrum_client::{Client, ElectrumApi}` — the same dependency
//! as wallet sync and `ElectrumFeeEstimator`. Connection and broadcast both run
//! inside `spawn_blocking` (the Electrum client is blocking I/O), mirroring
//! `wallet_service.rs::do_sync`.

use async_trait::async_trait;

use crate::application::tx_broadcaster::{is_already_known, TxBroadcastError, TxBroadcaster};

const SOURCE: &str = "Electrum";

/// Broadcasts commit+reveal via an Electrum server.
pub struct ElectrumBroadcaster {
    electrum_url: String,
}

impl ElectrumBroadcaster {
    pub fn new(electrum_url: impl Into<String>) -> Self {
        Self {
            electrum_url: electrum_url.into(),
        }
    }
}

fn err(message: impl Into<String>) -> TxBroadcastError {
    TxBroadcastError {
        source_name: SOURCE,
        message: message.into(),
    }
}

fn broadcast_one(
    client: &bdk_electrum::electrum_client::Client,
    hex: &str,
) -> Result<(), TxBroadcastError> {
    use bdk_electrum::electrum_client::ElectrumApi;

    let tx_bytes = hex::decode(hex).map_err(|e| err(format!("hex decode failed: {e}")))?;
    let tx: bitcoin::Transaction = bitcoin::consensus::deserialize(&tx_bytes)
        .map_err(|e| err(format!("tx deserialize failed: {e}")))?;
    match client.transaction_broadcast(&tx) {
        Ok(_) => Ok(()),
        Err(e) if is_already_known(&e.to_string()) => Ok(()),
        Err(e) => Err(err(e.to_string())),
    }
}

#[async_trait]
impl TxBroadcaster for ElectrumBroadcaster {
    fn name(&self) -> &'static str {
        SOURCE
    }

    async fn broadcast_pair(
        &self,
        commit_hex: &str,
        reveal_hex: &str,
    ) -> Result<(), TxBroadcastError> {
        let url = self.electrum_url.clone();
        let commit = commit_hex.to_string();
        let reveal = reveal_hex.to_string();

        tokio::task::spawn_blocking(move || {
            let client =
                bdk_electrum::electrum_client::Client::new(&url).map_err(|e| err(e.to_string()))?;
            broadcast_one(&client, &commit)?;
            broadcast_one(&client, &reveal)
        })
        .await
        .map_err(|e| err(format!("spawn_blocking panic: {e}")))?
    }

    async fn broadcast_one(&self, tx_hex: &str) -> Result<(), TxBroadcastError> {
        let url = self.electrum_url.clone();
        let hex = tx_hex.to_string();

        tokio::task::spawn_blocking(move || {
            let client =
                bdk_electrum::electrum_client::Client::new(&url).map_err(|e| err(e.to_string()))?;
            broadcast_one(&client, &hex)
        })
        .await
        .map_err(|e| err(format!("spawn_blocking panic: {e}")))?
    }
}
