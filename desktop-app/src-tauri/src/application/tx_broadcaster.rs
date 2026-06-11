//! Port (trait) for submitting the signed commit+reveal pair to the Bitcoin network.
//!
//! Infrastructure modules implement this trait; `submit_commit_then_reveal` uses it
//! to broadcast via Electrum first, Bitcoin node as fallback (M3).

use async_trait::async_trait;

/// Failure from one broadcaster. `message` carries the underlying cause verbatim
/// (connection refused, mempool rejection, ...).
#[derive(Debug, thiserror::Error)]
#[error("{source_name}: {message}")]
pub struct TxBroadcastError {
    pub source_name: &'static str,
    pub message: String,
}

/// Returns `true` if the error message indicates the transaction is already known
/// to the node/server — idempotency rule from spec §8.1: re-submission after a
/// partial earlier attempt must be treated as success.
pub fn is_already_known(msg: &str) -> bool {
    let lower = msg.to_lowercase();
    lower.contains("already") || lower.contains("duplicate")
}

/// Submit signed transactions to the Bitcoin network.
///
/// Implementations MUST treat "already in mempool / known" responses as
/// success — see [`is_already_known`].
#[async_trait]
pub trait TxBroadcaster: Send + Sync {
    fn name(&self) -> &'static str;

    /// Submit commit first, then reveal. Sequential submission is correct because
    /// the reveal spends the unconfirmed commit output; the node/Electrum server
    /// accepts in-mempool chained spends.
    async fn broadcast_pair(
        &self,
        commit_hex: &str,
        reveal_hex: &str,
    ) -> Result<(), TxBroadcastError>;

    /// Submit a single signed transaction (Phase 5 fee-bump replacement).
    /// Same idempotency rule as [`broadcast_pair`](Self::broadcast_pair):
    /// an "already known" response is success.
    async fn broadcast_one(&self, tx_hex: &str) -> Result<(), TxBroadcastError>;
}

/// Try each broadcaster in order for a single transaction; the first success wins
/// (same Electrum-first / node-fallback walk as the commit+reveal pair broadcast).
/// When every broadcaster fails, returns all accumulated errors so the caller can
/// surface every source verbatim.
pub async fn broadcast_single_with_fallback(
    broadcasters: &[std::sync::Arc<dyn TxBroadcaster>],
    tx_hex: &str,
) -> Result<(), Vec<TxBroadcastError>> {
    let mut errors: Vec<TxBroadcastError> = Vec::new();
    for b in broadcasters {
        match b.broadcast_one(tx_hex).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!(broadcaster = b.name(), error = %e, "single-tx broadcaster failed");
                errors.push(e);
            }
        }
    }
    Err(errors)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Test double: succeeds, or fails with a fixed message. Records every
    /// single-tx broadcast so tests can assert exactly what was submitted.
    pub struct MockBroadcaster {
        name: &'static str,
        error: Option<String>,
        sent: std::sync::Mutex<Vec<String>>,
    }

    impl MockBroadcaster {
        pub fn ok(name: &'static str) -> Self {
            Self {
                name,
                error: None,
                sent: std::sync::Mutex::new(Vec::new()),
            }
        }

        pub fn failing(name: &'static str, msg: &str) -> Self {
            Self {
                name,
                error: Some(msg.to_string()),
                sent: std::sync::Mutex::new(Vec::new()),
            }
        }

        /// Hexes submitted through `broadcast_one`, in order.
        pub fn sent_single(&self) -> Vec<String> {
            self.sent.lock().expect("mock lock").clone()
        }
    }

    #[async_trait]
    impl TxBroadcaster for MockBroadcaster {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn broadcast_pair(&self, _: &str, _: &str) -> Result<(), TxBroadcastError> {
            match &self.error {
                None => Ok(()),
                Some(msg) => Err(TxBroadcastError {
                    source_name: self.name,
                    message: msg.clone(),
                }),
            }
        }

        async fn broadcast_one(&self, tx_hex: &str) -> Result<(), TxBroadcastError> {
            self.sent
                .lock()
                .expect("mock lock")
                .push(tx_hex.to_string());
            match &self.error {
                None => Ok(()),
                Some(msg) => Err(TxBroadcastError {
                    source_name: self.name,
                    message: msg.clone(),
                }),
            }
        }
    }

    #[tokio::test]
    async fn broadcast_single_first_broadcaster_success_short_circuits() {
        let first = std::sync::Arc::new(MockBroadcaster::ok("Electrum"));
        let second = std::sync::Arc::new(MockBroadcaster::ok("Bitcoin node"));
        let chain: Vec<std::sync::Arc<dyn TxBroadcaster>> = vec![
            std::sync::Arc::clone(&first) as _,
            std::sync::Arc::clone(&second) as _,
        ];

        broadcast_single_with_fallback(&chain, "aabb")
            .await
            .unwrap();

        assert_eq!(first.sent_single(), vec!["aabb".to_string()]);
        assert!(
            second.sent_single().is_empty(),
            "fallback must not run after success"
        );
    }

    #[tokio::test]
    async fn broadcast_single_falls_back_when_first_fails() {
        let first = std::sync::Arc::new(MockBroadcaster::failing("Electrum", "connection refused"));
        let second = std::sync::Arc::new(MockBroadcaster::ok("Bitcoin node"));
        let chain: Vec<std::sync::Arc<dyn TxBroadcaster>> = vec![
            std::sync::Arc::clone(&first) as _,
            std::sync::Arc::clone(&second) as _,
        ];

        broadcast_single_with_fallback(&chain, "aabb")
            .await
            .unwrap();

        assert_eq!(second.sent_single(), vec!["aabb".to_string()]);
    }

    #[tokio::test]
    async fn broadcast_single_aggregates_errors_when_all_fail() {
        let chain: Vec<std::sync::Arc<dyn TxBroadcaster>> = vec![
            std::sync::Arc::new(MockBroadcaster::failing("Electrum", "connection refused")),
            std::sync::Arc::new(MockBroadcaster::failing("Bitcoin node", "insufficient fee")),
        ];

        let errors = broadcast_single_with_fallback(&chain, "aabb")
            .await
            .unwrap_err();

        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].source_name, "Electrum");
        assert!(errors[0].message.contains("connection refused"));
        assert_eq!(errors[1].source_name, "Bitcoin node");
        assert!(errors[1].message.contains("insufficient fee"));
    }

    #[test]
    fn is_already_known_matches_expected_phrases() {
        assert!(is_already_known("Transaction already in block chain"));
        assert!(is_already_known("txn-already-in-mempool"));
        assert!(is_already_known("txn-already-known"));
        assert!(is_already_known("duplicate transaction"));
        assert!(!is_already_known("insufficient fee"));
        assert!(!is_already_known("connection refused"));
    }
}
