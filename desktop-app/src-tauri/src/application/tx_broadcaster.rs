//! Port (trait) for submitting the signed commit+reveal pair to the Bitcoin network.
//!
//! Infrastructure modules implement this trait; `submit_commit_then_reveal` uses it
//! to broadcast via Electrum first, Bitcoin node as fallback (M3).

use async_trait::async_trait;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum TxBroadcastError {
    #[error("{source_name} rejected the transaction: {message}")]
    Rejected {
        source_name: &'static str,
        message: String,
    },
    #[error("{source_name} unavailable: {message}")]
    Unavailable {
        source_name: &'static str,
        message: String,
    },
}

impl TxBroadcastError {
    pub fn source_name(&self) -> &'static str {
        match self {
            Self::Rejected { source_name, .. } => source_name,
            Self::Unavailable { source_name, .. } => source_name,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Rejected { message, .. } => message,
            Self::Unavailable { message, .. } => message,
        }
    }
}

// ---------------------------------------------------------------------------
// TxBroadcaster trait
// ---------------------------------------------------------------------------

/// Submit a signed commit+reveal pair to the Bitcoin network.
///
/// Implementations MUST treat "already in mempool / known" responses as
/// success — idempotent re-submission after a partial earlier attempt.
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Test double that records calls and returns scripted results.
    pub struct MockBroadcaster {
        pub name: &'static str,
        /// If `Some(Err(...))`, returns that error; otherwise returns `Ok(())`.
        pub result: Mutex<Option<TxBroadcastError>>,
        pub calls: Mutex<Vec<(String, String)>>,
    }

    impl MockBroadcaster {
        pub fn ok(name: &'static str) -> Self {
            Self {
                name,
                result: Mutex::new(None),
                calls: Mutex::new(vec![]),
            }
        }

        pub fn unavailable(name: &'static str, msg: &str) -> Self {
            let msg = msg.to_string();
            Self {
                name,
                result: Mutex::new(Some(TxBroadcastError::Unavailable {
                    source_name: name,
                    message: msg,
                })),
                calls: Mutex::new(vec![]),
            }
        }
    }

    #[async_trait]
    impl TxBroadcaster for MockBroadcaster {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn broadcast_pair(
            &self,
            commit_hex: &str,
            reveal_hex: &str,
        ) -> Result<(), TxBroadcastError> {
            self.calls
                .lock()
                .unwrap()
                .push((commit_hex.to_string(), reveal_hex.to_string()));
            match &*self.result.lock().unwrap() {
                None => Ok(()),
                Some(TxBroadcastError::Unavailable {
                    source_name,
                    message,
                }) => Err(TxBroadcastError::Unavailable {
                    source_name,
                    message: message.clone(),
                }),
                Some(TxBroadcastError::Rejected {
                    source_name,
                    message,
                }) => Err(TxBroadcastError::Rejected {
                    source_name,
                    message: message.clone(),
                }),
            }
        }
    }

    #[tokio::test]
    async fn mock_ok_broadcaster_records_call() {
        let b = MockBroadcaster::ok("test");
        b.broadcast_pair("commit", "reveal").await.unwrap();
        let calls = b.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], ("commit".to_string(), "reveal".to_string()));
    }

    #[tokio::test]
    async fn mock_unavailable_broadcaster_returns_error() {
        let b = MockBroadcaster::unavailable("test", "down");
        let err = b.broadcast_pair("c", "r").await.unwrap_err();
        assert!(matches!(err, TxBroadcastError::Unavailable { .. }));
    }
}
