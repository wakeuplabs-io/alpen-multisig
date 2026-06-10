//! [`TxBroadcaster`] implementation over the Electrum protocol (M3).
//!
//! Uses `bdk_electrum::electrum_client::{Client, ElectrumApi}` — the same dependency
//! as wallet sync and `ElectrumFeeEstimator`. Each call opens a fresh connection
//! (the same pattern as wallet sync in `wallet_service.rs`).

use async_trait::async_trait;

use crate::application::tx_broadcaster::{TxBroadcastError, TxBroadcaster};

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

    fn connect(&self) -> Result<bdk_electrum::electrum_client::Client, TxBroadcastError> {
        bdk_electrum::electrum_client::Client::new(&self.electrum_url).map_err(|e| {
            TxBroadcastError::Unavailable {
                source_name: self.name(),
                message: e.to_string(),
            }
        })
    }
}

/// Returns `true` if the error message indicates the transaction is already known
/// to the node/server — idempotency rule from spec §8.1.
fn is_already_known(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("already") || lower.contains("duplicate")
}

#[async_trait]
impl TxBroadcaster for ElectrumBroadcaster {
    fn name(&self) -> &'static str {
        "Electrum"
    }

    async fn broadcast_pair(
        &self,
        commit_hex: &str,
        reveal_hex: &str,
    ) -> Result<(), TxBroadcastError> {
        use bdk_electrum::electrum_client::ElectrumApi;

        let url = self.electrum_url.clone();
        let commit = commit_hex.to_string();
        let reveal = reveal_hex.to_string();
        let client = self.connect()?;

        tokio::task::spawn_blocking(move || {
            let broadcast_one =
                |hex: &str| -> Result<(), TxBroadcastError> {
                    let tx_bytes = hex::decode(hex).map_err(|e| TxBroadcastError::Rejected {
                        source_name: "Electrum",
                        message: format!("hex decode failed: {e}"),
                    })?;
                    let tx: bitcoin::Transaction = bitcoin::consensus::deserialize(&tx_bytes)
                        .map_err(|e| TxBroadcastError::Rejected {
                            source_name: "Electrum",
                            message: format!("tx deserialize failed: {e}"),
                        })?;
                    match client.transaction_broadcast(&tx) {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            let msg = e.to_string();
                            if is_already_known(&msg) {
                                Ok(())
                            } else {
                                Err(TxBroadcastError::Rejected {
                                    source_name: "Electrum",
                                    message: msg,
                                })
                            }
                        }
                    }
                };

            broadcast_one(&commit)?;
            broadcast_one(&reveal)
        })
        .await
        .map_err(|e| TxBroadcastError::Unavailable {
            source_name: "Electrum",
            message: format!("spawn_blocking panic at {url}: {e}"),
        })?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_already_known_matches_expected_phrases() {
        assert!(is_already_known("Transaction already in block chain"));
        assert!(is_already_known("txn-already-in-mempool"));
        assert!(is_already_known("txn-already-known"));
        assert!(is_already_known("duplicate transaction"));
        assert!(!is_already_known("insufficient fee"));
        assert!(!is_already_known("connection refused"));
    }

    #[tokio::test]
    async fn unreachable_server_maps_to_unavailable() {
        let b = ElectrumBroadcaster::new("tcp://127.0.0.1:1");
        let err = b.broadcast_pair("aabbcc", "ddeeff").await.unwrap_err();
        assert!(matches!(err, TxBroadcastError::Unavailable { .. }));
    }
}
