//! [`TxBroadcaster`] implementation backed by the Bitcoin node JSON-RPC (M3).
//!
//! Tries `submitpackage` first (available in Bitcoin Core 25+); falls back to
//! sequential `sendrawtransaction` when the method is unknown — preserving the
//! behavior that existed in `proposals.rs::submit_commit_then_reveal` before M3.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::tx_broadcaster::{TxBroadcastError, TxBroadcaster};
use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;

/// Returns `true` when the RPC error indicates `submitpackage` is not available.
fn is_unknown_method(err: &str) -> bool {
    // Bitcoin Core returns -32601 for unknown methods; the message contains
    // "Method not found" or similar RPC-level text.
    err.contains("Method not found") || err.contains("-32601")
}

/// Returns `true` if the transaction is already present in the mempool or chain.
fn is_already_known(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("already") || lower.contains("duplicate")
}

/// `TxBroadcaster` that uses the Bitcoin node RPC.
pub struct NodeBroadcaster {
    rpc: Arc<dyn BitcoinRpcClient>,
}

impl NodeBroadcaster {
    pub fn new(rpc: Arc<dyn BitcoinRpcClient>) -> Self {
        Self { rpc }
    }

    fn rpc_err_to_broadcast_err(&self, e: String) -> TxBroadcastError {
        TxBroadcastError::Rejected {
            source_name: self.name(),
            message: e,
        }
    }
}

#[async_trait]
impl TxBroadcaster for NodeBroadcaster {
    fn name(&self) -> &'static str {
        "Bitcoin node"
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
            Err(e) => {
                if is_already_known(&e) {
                    return Ok(());
                }
                return Err(self.rpc_err_to_broadcast_err(e));
            }
        }

        // Sequential fallback: send commit first, then reveal.
        match self.rpc.send_raw_transaction(commit_hex).await {
            Ok(_) => {}
            Err(e) if is_already_known(&e) => {}
            Err(e) => return Err(self.rpc_err_to_broadcast_err(e)),
        }
        match self.rpc.send_raw_transaction(reveal_hex).await {
            Ok(_) => {}
            Err(e) if is_already_known(&e) => {}
            Err(e) => return Err(self.rpc_err_to_broadcast_err(e)),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use bitcoin::Transaction;

    struct AlwaysOkRpc;

    #[async_trait]
    impl BitcoinRpcClient for AlwaysOkRpc {
        async fn send_raw_transaction(&self, _: &str) -> Result<String, String> {
            Ok("txid".to_string())
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
        async fn submit_package(&self, _: &[String]) -> Result<(), String> {
            Err("Method not found (-32601)".to_string())
        }
    }

    struct RejectRpc;

    #[async_trait]
    impl BitcoinRpcClient for RejectRpc {
        async fn send_raw_transaction(&self, _: &str) -> Result<String, String> {
            Err("bad-txns-inputs-missingorspent".to_string())
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
        async fn submit_package(&self, _: &[String]) -> Result<(), String> {
            Err("Method not found (-32601)".to_string())
        }
    }

    #[tokio::test]
    async fn sequential_fallback_used_when_submit_package_unknown() {
        let b = NodeBroadcaster::new(Arc::new(AlwaysOkRpc));
        b.broadcast_pair("aa", "bb").await.unwrap();
    }

    #[tokio::test]
    async fn rejection_propagates_as_error() {
        let b = NodeBroadcaster::new(Arc::new(RejectRpc));
        let err = b.broadcast_pair("aa", "bb").await.unwrap_err();
        assert!(matches!(err, TxBroadcastError::Rejected { .. }));
    }

    #[test]
    fn is_unknown_method_matches_expected_errors() {
        assert!(is_unknown_method("Method not found (-32601)"));
        assert!(is_unknown_method("Method not found"));
        assert!(is_unknown_method("-32601"));
        assert!(!is_unknown_method("insufficient fee"));
    }
}
