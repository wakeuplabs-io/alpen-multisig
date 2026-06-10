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

/// Submit a signed commit+reveal pair to the Bitcoin network.
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
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Test double: succeeds, or fails with a fixed message.
    pub struct MockBroadcaster {
        name: &'static str,
        error: Option<String>,
    }

    impl MockBroadcaster {
        pub fn ok(name: &'static str) -> Self {
            Self { name, error: None }
        }

        pub fn failing(name: &'static str, msg: &str) -> Self {
            Self {
                name,
                error: Some(msg.to_string()),
            }
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
