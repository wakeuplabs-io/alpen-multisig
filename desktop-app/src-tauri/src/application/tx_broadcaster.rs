//! Port (trait) for submitting the signed commit+reveal pair to the Bitcoin network.
//!
//! Infrastructure modules implement this trait; `submit_commit_then_reveal` uses it
//! to broadcast via Electrum first, Bitcoin node as fallback (M3).
//!
//! Every failure says how far the transaction got (#516): the caller decides from the
//! broadcaster's own answer whether the coins it spends may be reused, never from a later lookup.

use async_trait::async_trait;

/// How a broadcast that did not succeed ended at one source (#516).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// The source answered with an explicit rejection: it does not hold the transaction.
    Rejected,
    /// The connection never opened: nothing reached the source.
    NotDelivered,
    /// Anything else — a timeout, a connection dropped after sending, an answer that is not a
    /// rejection. The source may hold the transaction.
    Ambiguous,
}

/// Where one transaction stands after a broadcast, at one source or across all of them (#516).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxOutcome {
    /// Accepted (or already known).
    Accepted,
    Rejected,
    NotDelivered,
    Ambiguous,
}

impl From<FailureKind> for TxOutcome {
    fn from(kind: FailureKind) -> Self {
        match kind {
            FailureKind::Rejected => TxOutcome::Rejected,
            FailureKind::NotDelivered => TxOutcome::NotDelivered,
            FailureKind::Ambiguous => TxOutcome::Ambiguous,
        }
    }
}

impl TxOutcome {
    /// Combines one transaction's outcome at every source: any acceptance wins; otherwise any
    /// ambiguity keeps it possibly live; only when no source may hold it is it `Rejected` (at
    /// least one source refused it) or `NotDelivered` (no source was ever reached).
    pub fn combine(outcomes: impl IntoIterator<Item = TxOutcome>) -> TxOutcome {
        let mut combined = TxOutcome::NotDelivered;
        for outcome in outcomes {
            combined = match (combined, outcome) {
                (TxOutcome::Accepted, _) | (_, TxOutcome::Accepted) => TxOutcome::Accepted,
                (TxOutcome::Ambiguous, _) | (_, TxOutcome::Ambiguous) => TxOutcome::Ambiguous,
                (TxOutcome::Rejected, _) | (_, TxOutcome::Rejected) => TxOutcome::Rejected,
                _ => TxOutcome::NotDelivered,
            };
        }
        combined
    }
}

/// Failure from one broadcaster. `message` carries the underlying cause verbatim
/// (connection refused, mempool rejection, ...).
#[derive(Debug, thiserror::Error)]
#[error("{source_name}: {message}")]
pub struct TxBroadcastError {
    pub source_name: &'static str,
    pub message: String,
    pub kind: FailureKind,
}

/// A pair broadcast that did not fully land at one source.
#[derive(Debug)]
pub struct PairBroadcastError {
    /// The source accepted the commit, so `error` is about the reveal. Otherwise `error` is
    /// about the commit (and the reveal was not accepted either).
    pub commit_accepted: bool,
    pub error: TxBroadcastError,
}

impl PairBroadcastError {
    /// The commit itself failed.
    pub fn at_commit(error: TxBroadcastError) -> Self {
        Self {
            commit_accepted: false,
            error,
        }
    }

    /// The commit landed; the reveal failed.
    pub fn at_reveal(error: TxBroadcastError) -> Self {
        Self {
            commit_accepted: true,
            error,
        }
    }

    fn commit_outcome(&self) -> TxOutcome {
        if self.commit_accepted {
            TxOutcome::Accepted
        } else {
            self.error.kind.into()
        }
    }
}

/// Every broadcaster failed. `outcome` is the transaction's combined [`TxOutcome`] — for a
/// pair, the commit's (`Accepted` then means only the reveal is missing).
#[derive(Debug)]
pub struct AllSourcesFailed {
    pub outcome: TxOutcome,
    pub errors: Vec<TxBroadcastError>,
}

impl AllSourcesFailed {
    /// Every source's error, `source: message`, for logs and user-facing detail.
    pub fn message(&self) -> String {
        self.errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// `(source, message)` pairs, the shape the IPC errors carry.
    pub fn by_source(&self) -> Vec<(String, String)> {
        self.errors
            .iter()
            .map(|e| (e.source_name.to_string(), e.message.clone()))
            .collect()
    }
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
/// success — see [`is_already_known`] — and classify every failure by [`FailureKind`].
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
    ) -> Result<(), PairBroadcastError>;

    /// Submit a single signed transaction (Phase 5 fee-bump replacement).
    /// Same idempotency rule as [`broadcast_pair`](Self::broadcast_pair):
    /// an "already known" response is success.
    async fn broadcast_one(&self, tx_hex: &str) -> Result<(), TxBroadcastError>;
}

/// Try each broadcaster in order for a single transaction; the first success wins
/// (same Electrum-first / node-fallback walk as the commit+reveal pair broadcast).
/// When every broadcaster fails, returns all accumulated errors — so the caller can
/// surface every source verbatim — and the combined outcome.
pub async fn broadcast_single_with_fallback(
    broadcasters: &[std::sync::Arc<dyn TxBroadcaster>],
    tx_hex: &str,
) -> Result<(), AllSourcesFailed> {
    let mut errors: Vec<TxBroadcastError> = Vec::new();
    for b in broadcasters {
        match b.broadcast_one(tx_hex).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!(broadcaster = b.name(), error = %e, kind = ?e.kind, "single-tx broadcaster failed");
                errors.push(e);
            }
        }
    }
    Err(AllSourcesFailed {
        outcome: TxOutcome::combine(errors.iter().map(|e| e.kind.into())),
        errors,
    })
}

/// Try each broadcaster in order for the commit+reveal pair; the first source that lands both
/// wins. A source that took only the commit does not stop the walk — the next may land the
/// reveal. On total failure the outcome is the commit's, combined across sources.
pub async fn broadcast_pair_with_fallback(
    broadcasters: &[std::sync::Arc<dyn TxBroadcaster>],
    commit_hex: &str,
    reveal_hex: &str,
) -> Result<(), AllSourcesFailed> {
    let mut commit_outcomes = Vec::new();
    let mut errors = Vec::new();
    for b in broadcasters {
        match b.broadcast_pair(commit_hex, reveal_hex).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                tracing::warn!(broadcaster = b.name(), error = %e.error, commit_accepted = e.commit_accepted, kind = ?e.error.kind, "broadcaster failed");
                commit_outcomes.push(e.commit_outcome());
                errors.push(e.error);
            }
        }
    }
    Err(AllSourcesFailed {
        outcome: TxOutcome::combine(commit_outcomes),
        errors,
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::Arc;

    /// Test double: succeeds, or fails with a fixed [`FailureKind`]. Records every
    /// single-tx broadcast so tests can assert exactly what was submitted.
    pub struct MockBroadcaster {
        name: &'static str,
        failure: Option<(FailureKind, String)>,
        /// Failing mode, pairs only: the commit lands before the reveal fails.
        commit_lands: bool,
        sent: std::sync::Mutex<Vec<String>>,
    }

    impl MockBroadcaster {
        pub fn ok(name: &'static str) -> Self {
            Self {
                name,
                failure: None,
                commit_lands: false,
                sent: std::sync::Mutex::new(Vec::new()),
            }
        }

        /// Fails every broadcast with `kind`.
        pub fn failing_with(name: &'static str, kind: FailureKind, msg: &str) -> Self {
            Self {
                failure: Some((kind, msg.to_string())),
                ..Self::ok(name)
            }
        }

        /// The source answers with an explicit rejection.
        pub fn failing(name: &'static str, msg: &str) -> Self {
            Self::failing_with(name, FailureKind::Rejected, msg)
        }

        /// The connection never opens.
        pub fn unreachable(name: &'static str) -> Self {
            Self::failing_with(name, FailureKind::NotDelivered, "connection refused")
        }

        /// The request went out and no answer came back.
        pub fn ambiguous(name: &'static str) -> Self {
            Self::failing_with(name, FailureKind::Ambiguous, "operation timed out")
        }

        /// Pairs: the commit is accepted, then the reveal fails with `kind`.
        pub fn landing_commit_then(name: &'static str, kind: FailureKind, msg: &str) -> Self {
            Self {
                commit_lands: true,
                ..Self::failing_with(name, kind, msg)
            }
        }

        /// Hexes submitted through `broadcast_one`, in order.
        pub fn sent_single(&self) -> Vec<String> {
            self.sent.lock().expect("mock lock").clone()
        }

        fn result(&self) -> Result<(), TxBroadcastError> {
            match &self.failure {
                None => Ok(()),
                Some((kind, msg)) => Err(TxBroadcastError {
                    source_name: self.name,
                    message: msg.clone(),
                    kind: *kind,
                }),
            }
        }
    }

    #[async_trait]
    impl TxBroadcaster for MockBroadcaster {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn broadcast_pair(&self, _: &str, _: &str) -> Result<(), PairBroadcastError> {
            self.result().map_err(|e| {
                if self.commit_lands {
                    PairBroadcastError::at_reveal(e)
                } else {
                    PairBroadcastError::at_commit(e)
                }
            })
        }

        async fn broadcast_one(&self, tx_hex: &str) -> Result<(), TxBroadcastError> {
            self.sent
                .lock()
                .expect("mock lock")
                .push(tx_hex.to_string());
            self.result()
        }
    }

    fn chain(mocks: Vec<MockBroadcaster>) -> Vec<Arc<dyn TxBroadcaster>> {
        mocks
            .into_iter()
            .map(|m| Arc::new(m) as Arc<dyn TxBroadcaster>)
            .collect()
    }

    /// #516: one tx's outcomes across sources. Acceptance anywhere wins; a source that may
    /// hold it keeps it possibly live; only then does one explicit rejection decide.
    #[test]
    fn outcomes_combine_across_sources() {
        use TxOutcome::{Accepted, Ambiguous, NotDelivered, Rejected};
        let cases: Vec<(Vec<TxOutcome>, TxOutcome)> = vec![
            (vec![Rejected, Accepted], Accepted),
            (vec![Ambiguous, Accepted], Accepted),
            (vec![Rejected, Ambiguous], Ambiguous),
            (vec![NotDelivered, Ambiguous], Ambiguous),
            (vec![NotDelivered, NotDelivered], NotDelivered),
            (vec![], NotDelivered),
            (vec![NotDelivered, Rejected], Rejected),
            (vec![Rejected, Rejected], Rejected),
        ];
        for (outcomes, expected) in cases {
            assert_eq!(
                TxOutcome::combine(outcomes.clone()),
                expected,
                "{outcomes:?}"
            );
        }
    }

    #[tokio::test]
    async fn broadcast_single_first_broadcaster_success_short_circuits() {
        let first = Arc::new(MockBroadcaster::ok("Electrum"));
        let second = Arc::new(MockBroadcaster::ok("Bitcoin node"));
        let chain: Vec<Arc<dyn TxBroadcaster>> =
            vec![Arc::clone(&first) as _, Arc::clone(&second) as _];

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
        let first = Arc::new(MockBroadcaster::failing("Electrum", "connection refused"));
        let second = Arc::new(MockBroadcaster::ok("Bitcoin node"));
        let chain: Vec<Arc<dyn TxBroadcaster>> =
            vec![Arc::clone(&first) as _, Arc::clone(&second) as _];

        broadcast_single_with_fallback(&chain, "aabb")
            .await
            .unwrap();

        assert_eq!(second.sent_single(), vec!["aabb".to_string()]);
    }

    #[tokio::test]
    async fn broadcast_single_aggregates_errors_and_outcome_when_all_fail() {
        let chain = chain(vec![
            MockBroadcaster::unreachable("Electrum"),
            MockBroadcaster::failing("Bitcoin node", "insufficient fee"),
        ]);

        let failure = broadcast_single_with_fallback(&chain, "aabb")
            .await
            .unwrap_err();

        assert_eq!(failure.outcome, TxOutcome::Rejected);
        assert_eq!(
            failure.by_source(),
            vec![
                ("Electrum".to_string(), "connection refused".to_string()),
                ("Bitcoin node".to_string(), "insufficient fee".to_string()),
            ]
        );
    }

    /// #516: a source that took the commit but not the reveal leaves the commit `Accepted`,
    /// whatever the next source answers.
    #[tokio::test]
    async fn pair_commit_accepted_by_one_source_stays_accepted() {
        let chain = chain(vec![
            MockBroadcaster::landing_commit_then(
                "Bitcoin node",
                FailureKind::NotDelivered,
                "connection refused",
            ),
            MockBroadcaster::failing("Electrum", "missing inputs"),
        ]);

        let failure = broadcast_pair_with_fallback(&chain, "c0", "r0")
            .await
            .unwrap_err();

        assert_eq!(failure.outcome, TxOutcome::Accepted);
        assert_eq!(failure.errors.len(), 2);
    }

    #[tokio::test]
    async fn pair_lands_through_the_next_source_after_a_partial_one() {
        let chain = chain(vec![
            MockBroadcaster::landing_commit_then("Electrum", FailureKind::Rejected, "too-long"),
            MockBroadcaster::ok("Bitcoin node"),
        ]);

        broadcast_pair_with_fallback(&chain, "c0", "r0")
            .await
            .expect("the second source lands both");
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
