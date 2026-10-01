//! [`TxBroadcaster`] implementation backed by the Bitcoin node JSON-RPC (M3).
//!
//! Tries `submitpackage` first (available in Bitcoin Core 25+); falls back to
//! sequential `sendrawtransaction` whenever the node answered `submitpackage` without accepting
//! the whole package — an unknown method, a package error, or a non-success `package_msg` (#516).

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::tx_broadcaster::{
    is_already_known, FailureKind, PairBroadcastError, TxBroadcastError, TxBroadcaster,
};
use crate::infrastructure::bitcoin_rpc::{BitcoinRpcClient, RpcError, RpcFailureKind};

const SOURCE: &str = "Bitcoin node";

fn err(e: RpcError, kind: FailureKind) -> TxBroadcastError {
    TxBroadcastError {
        source_name: SOURCE,
        message: e.message,
        kind,
    }
}

/// A single-tx call's failure (#516): the node's JSON-RPC error answer is a rejection.
fn tx_failure(e: RpcError) -> TxBroadcastError {
    let kind = match e.kind {
        RpcFailureKind::Answered => FailureKind::Rejected,
        RpcFailureKind::NotConnected => FailureKind::NotDelivered,
        RpcFailureKind::Unknown => FailureKind::Ambiguous,
    };
    err(e, kind)
}

/// A `submitpackage` failure the node did not answer (#516): the package may be in the mempool,
/// unless the connection never opened.
fn unanswered_package_failure(e: RpcError) -> TxBroadcastError {
    let kind = match e.kind {
        RpcFailureKind::NotConnected => FailureKind::NotDelivered,
        _ => FailureKind::Ambiguous,
    };
    err(e, kind)
}

/// `TxBroadcaster` that uses the Bitcoin node RPC.
pub struct NodeBroadcaster {
    rpc: Arc<dyn BitcoinRpcClient>,
}

impl NodeBroadcaster {
    pub fn new(rpc: Arc<dyn BitcoinRpcClient>) -> Self {
        Self { rpc }
    }

    async fn send(&self, hex: &str) -> Result<(), TxBroadcastError> {
        match self.rpc.send_raw_transaction(hex).await {
            Ok(_) => Ok(()),
            Err(e) if is_already_known(&e.message) => Ok(()),
            Err(e) => Err(tx_failure(e)),
        }
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
    ) -> Result<(), PairBroadcastError> {
        match self
            .rpc
            .submit_package(&[commit_hex.to_string(), reveal_hex.to_string()])
            .await
        {
            Ok(()) => return Ok(()),
            // The node answered without taking the whole package: an unknown method, a package
            // error, or a non-success `package_msg`. Part of it may be in the mempool, so ask
            // again tx by tx — each answer below is definitive, "already known" included.
            Err(e) if e.kind == RpcFailureKind::Answered => {
                tracing::info!(error = %e, "submitpackage did not take the package; sending tx by tx");
            }
            Err(e) => return Err(PairBroadcastError::at_commit(unanswered_package_failure(e))),
        }

        // Sequential fallback: send commit first, then reveal. Each tx keeps its own outcome:
        // a commit accepted before the reveal fails stays accepted.
        self.send(commit_hex)
            .await
            .map_err(PairBroadcastError::at_commit)?;
        self.send(reveal_hex)
            .await
            .map_err(PairBroadcastError::at_reveal)
    }

    async fn broadcast_one(&self, tx_hex: &str) -> Result<(), TxBroadcastError> {
        self.send(tx_hex).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use bitcoin::Transaction;
    use std::sync::Mutex;

    /// Stub node: `submitpackage` and each `sendrawtransaction` answer from a script.
    struct StubRpc {
        package_result: Result<(), RpcError>,
        /// One answer per `sendrawtransaction` call, in order; the last one repeats.
        send_results: Mutex<Vec<Result<(), RpcError>>>,
    }

    impl StubRpc {
        fn without_package(send_results: Vec<Result<(), RpcError>>) -> Self {
            Self {
                package_result: Err(RpcError::answered(Some(-32601), "Method not found")),
                send_results: Mutex::new(send_results),
            }
        }

        fn sending(result: Result<(), RpcError>) -> Self {
            Self::without_package(vec![result])
        }
    }

    #[async_trait]
    impl BitcoinRpcClient for StubRpc {
        async fn send_raw_transaction(&self, _: &str) -> Result<String, RpcError> {
            let mut results = self.send_results.lock().unwrap();
            let result = if results.len() > 1 {
                results.remove(0)
            } else {
                results[0].clone()
            };
            result.map(|_| "txid".to_string())
        }
        async fn submit_package(&self, _: &[String]) -> Result<(), RpcError> {
            self.package_result.clone()
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
        async fn get_transaction_depth(
            &self,
            _: &str,
        ) -> Result<u32, crate::infrastructure::bitcoin_rpc::RpcError> {
            unimplemented!()
        }
        async fn get_raw_transaction(&self, _: &str) -> Result<Transaction, String> {
            unimplemented!()
        }
        async fn get_block_count(&self) -> Result<u64, String> {
            unimplemented!()
        }
    }

    fn node(rpc: StubRpc) -> NodeBroadcaster {
        NodeBroadcaster::new(Arc::new(rpc))
    }

    #[tokio::test]
    async fn sequential_fallback_used_when_submit_package_unknown() {
        node(StubRpc::sending(Ok(())))
            .broadcast_pair("aa", "bb")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn broadcast_one_treats_already_known_as_success() {
        node(StubRpc::sending(Err(RpcError::answered(
            Some(-27),
            "txn-already-in-mempool",
        ))))
        .broadcast_one("aa")
        .await
        .unwrap();
    }

    /// #516: a JSON-RPC error answer is a rejection; a connection that never opened delivered
    /// nothing; anything else (timeout, dropped connection) may have landed.
    #[tokio::test]
    async fn single_tx_failures_are_classified_from_the_rpc_error() {
        let cases = [
            (
                RpcError::answered(Some(-26), "insufficient fee"),
                FailureKind::Rejected,
            ),
            (
                RpcError::not_connected("bitcoin rpc send failed: tcp connect error"),
                FailureKind::NotDelivered,
            ),
            (
                RpcError::unknown("bitcoin rpc send failed: operation timed out"),
                FailureKind::Ambiguous,
            ),
        ];
        for (rpc_error, expected) in cases {
            let message = rpc_error.message.clone();
            let e = node(StubRpc::sending(Err(rpc_error)))
                .broadcast_one("aa")
                .await
                .unwrap_err();
            assert_eq!((e.source_name, e.kind), (SOURCE, expected), "{message}");
            assert_eq!(e.message, message);
        }
    }

    /// #516: an answered `submitpackage` error (or a non-success `package_msg`) says nothing
    /// definite about each tx — part of the package may be in the mempool — so the node is asked
    /// again tx by tx, and each `sendrawtransaction` answer is definitive ("already known" is an
    /// acceptance). Only a transport failure of `submitpackage` itself stays ambiguous.
    #[tokio::test]
    async fn submit_package_failures_are_settled_tx_by_tx() {
        type Expected = Result<(), (bool, FailureKind)>;
        let already_in_mempool = || RpcError::answered(Some(-27), "txn-already-in-mempool");
        type Case = (RpcError, Vec<Result<(), RpcError>>, Expected);
        let cases: Vec<Case> = vec![
            (
                RpcError::answered(Some(-26), "package-mempool-limits"),
                vec![Err(already_in_mempool()), Ok(())],
                Ok(()),
            ),
            (
                RpcError::answered(Some(-25), "package-not-validated"),
                vec![
                    Err(already_in_mempool()),
                    Err(RpcError::answered(Some(-26), "min relay fee not met")),
                ],
                Err((true, FailureKind::Rejected)),
            ),
            (
                RpcError::answered(Some(-25), "package-not-validated"),
                vec![Err(RpcError::answered(
                    Some(-25),
                    "bad-txns-inputs-missingorspent",
                ))],
                Err((false, FailureKind::Rejected)),
            ),
            (
                RpcError::answered(None, "submitpackage: package_msg: transaction failed"),
                vec![Ok(())],
                Ok(()),
            ),
            (
                RpcError::unknown("bitcoin rpc send failed: operation timed out"),
                vec![Ok(())],
                Err((false, FailureKind::Ambiguous)),
            ),
            (
                RpcError::not_connected("tcp connect error"),
                vec![Ok(())],
                Err((false, FailureKind::NotDelivered)),
            ),
        ];
        for (package_error, sends, expected) in cases {
            let label = package_error.message.clone();
            let rpc = StubRpc {
                package_result: Err(package_error),
                send_results: Mutex::new(sends),
            };
            let result = node(rpc)
                .broadcast_pair("aa", "bb")
                .await
                .map_err(|e| (e.commit_accepted, e.error.kind));
            assert_eq!(result, expected, "{label}");
        }
    }

    /// #516: in the sequential fallback each tx keeps its own outcome — a commit the node
    /// accepted stays accepted when the reveal then hits a connection error.
    #[tokio::test]
    async fn sequential_fallback_keeps_a_commit_accepted_before_the_reveal_fails() {
        let e = node(StubRpc::without_package(vec![
            Ok(()),
            Err(RpcError::not_connected("tcp connect error")),
        ]))
        .broadcast_pair("aa", "bb")
        .await
        .unwrap_err();

        assert!(e.commit_accepted, "the commit was accepted");
        assert_eq!(e.error.kind, FailureKind::NotDelivered);
    }

    #[tokio::test]
    async fn sequential_fallback_rejected_commit_is_reported_at_the_commit() {
        let e = node(StubRpc::sending(Err(RpcError::answered(
            Some(-25),
            "bad-txns-inputs-missingorspent",
        ))))
        .broadcast_pair("aa", "bb")
        .await
        .unwrap_err();

        assert_eq!(
            (e.commit_accepted, e.error.kind),
            (false, FailureKind::Rejected)
        );
        assert!(e.error.message.contains("bad-txns"));
    }
}
