//! [`TxBroadcaster`] implementation backed by the Bitcoin node JSON-RPC (M3).
//!
//! Tries `submitpackage` first (available in Bitcoin Core 25+); falls back to
//! sequential `sendrawtransaction` when the method is unknown — preserving the
//! behavior that existed in `proposals.rs::submit_commit_then_reveal` before M3.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::tx_broadcaster::{is_already_known, TxBroadcastError, TxBroadcaster};
use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;

const SOURCE: &str = "Bitcoin node";

/// Returns `true` when the RPC error indicates `submitpackage` is not available.
/// Bitcoin Core returns -32601 for unknown methods.
fn is_unknown_method(err: &str) -> bool {
    err.contains("-32601") || err.to_lowercase().contains("method not found")
}

fn err(message: String) -> TxBroadcastError {
    TxBroadcastError {
        source_name: SOURCE,
        message,
    }
}

/// `TxBroadcaster` that uses the Bitcoin node RPC.
pub struct NodeBroadcaster {
    rpc: Arc<dyn BitcoinRpcClient>,
}

impl NodeBroadcaster {
    pub fn new(rpc: Arc<dyn BitcoinRpcClient>) -> Self {
        Self { rpc }
    }
}

#[async_trait]
impl TxBroadcaster for NodeBroadcaster {
    fn name(&self) -> &'static str {
        SOURCE
    }

    async fn broadcast_pair(
        &self,
        commit_hex: &str,
        reveal_hex: &str,
    ) -> Result<(), TxBroadcastError> {
        match self
            .rpc
            .submit_package(&[commit_hex.to_string(), reveal_hex.to_string()])
            .await
        {
            Ok(()) => return Ok(()),
            Err(ref e) if is_unknown_method(e) => {
                // Fall through to sequential broadcast below.
            }
            Err(e) if is_already_known(&e) => return Ok(()),
            Err(e) => return Err(err(e)),
        }

        // Sequential fallback: send commit first, then reveal.
        for hex in [commit_hex, reveal_hex] {
            match self.rpc.send_raw_transaction(hex).await {
                Ok(_) => {}
                Err(e) if is_already_known(&e) => {}
                Err(e) => return Err(err(e)),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use bitcoin::Transaction;

    /// Stub node without `submitpackage`; `sendrawtransaction` outcome is scripted.
    struct StubRpc {
        send_raw_result: Result<(), &'static str>,
    }

    #[async_trait]
    impl BitcoinRpcClient for StubRpc {
        async fn send_raw_transaction(&self, _: &str) -> Result<String, String> {
            self.send_raw_result
                .map(|_| "txid".to_string())
                .map_err(String::from)
        }
        async fn submit_package(&self, _: &[String]) -> Result<(), String> {
            Err("Method not found (-32601)".to_string())
        }
        async fn get_transaction_confirmations(&self, _: &str) -> Result<u32, String> {
            unimplemented!()
        }
        async fn estimate_smart_fee_sat_per_kvb(&self, _: u16) -> Result<u64, String> {
            unimplemented!()
        }
        async fn min_relay_sat_per_kvb(&self) -> Result<u64, String> {
            unimplemented!()
        }
        async fn get_raw_transaction(&self, _: &str) -> Result<Transaction, String> {
            unimplemented!()
        }
        async fn get_block_count(&self) -> Result<u64, String> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn sequential_fallback_used_when_submit_package_unknown() {
        let b = NodeBroadcaster::new(Arc::new(StubRpc {
            send_raw_result: Ok(()),
        }));
        b.broadcast_pair("aa", "bb").await.unwrap();
    }

    #[tokio::test]
    async fn rejection_propagates_as_error() {
        let b = NodeBroadcaster::new(Arc::new(StubRpc {
            send_raw_result: Err("bad-txns-inputs-missingorspent"),
        }));
        let e = b.broadcast_pair("aa", "bb").await.unwrap_err();
        assert!(e.message.contains("bad-txns"));
    }

    #[test]
    fn is_unknown_method_matches_expected_errors() {
        assert!(is_unknown_method("Method not found (-32601)"));
        assert!(is_unknown_method("method not found"));
        assert!(is_unknown_method("-32601"));
        assert!(!is_unknown_method("insufficient fee"));
    }
}
