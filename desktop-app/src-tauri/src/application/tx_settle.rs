//! Settling transactions whose broadcast got no definitive answer (#516).
//!
//! A broadcaster's answer settles most broadcasts on the spot (see `tx_broadcaster`). What it
//! leaves open — a send or bundle that may be live (`Ambiguous`), a bundle nothing could deliver
//! (`NotDelivered`, handed over for a manual broadcast), a commit whose reveal was not accepted —
//! is settled here, by asking every configured source whether it holds the transaction.
//!
//! One rule for every open transaction:
//!
//! - **Found** at any source → it is live: record it, drop its reservation.
//! - **Absent** — every source answered that it does not hold it — on [`AbsenceWindow::checks`]
//!   consecutive checks spanning at least [`AbsenceWindow::span`] → it is gone: release its coins.
//! - Anything else (**Unknown**: a source did not answer) changes nothing and restarts the count.
//!
//! The window absorbs propagation and indexer lag: a transaction a source has not indexed yet is
//! "not found" for seconds, never for ten minutes across three checks.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bitcoin::{Transaction, Txid};

/// One source's answer to "do you hold this transaction?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lookup {
    /// In the mempool or in a block. `confirmed` is `true` only when the source knows it is mined.
    Found { confirmed: bool },
    /// The source answered, and it does not hold the transaction.
    NotFound,
    /// No usable answer: a transport error, a timeout, an error the source did not classify.
    Unanswered,
}

/// A transaction to look up. Sources that index by output script need the transaction itself.
#[derive(Debug, Clone)]
pub struct TrackedTx {
    pub txid: Txid,
    pub tx: Option<Transaction>,
}

impl TrackedTx {
    pub fn of(tx: &Transaction) -> Self {
        Self {
            txid: tx.compute_txid(),
            tx: Some(tx.clone()),
        }
    }
}

/// Port: a source that can tell whether it holds a transaction (the node, the Electrum server).
#[async_trait]
pub trait TxLookup: Send + Sync {
    fn name(&self) -> &'static str;
    async fn lookup(&self, tracked: &TrackedTx) -> Lookup;
}

/// Where one transaction stands across every configured source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// At least one source holds it; `confirmed` if any source knows it is mined.
    Found { confirmed: bool },
    /// Every source answered, and none holds it.
    Absent,
    /// No source holds it, and at least one did not answer (or there is no source).
    Unknown,
}

impl Presence {
    /// Found if any source found it; Absent only if every source answered `NotFound`.
    pub fn combine(lookups: impl IntoIterator<Item = Lookup>) -> Presence {
        let mut found: Option<bool> = None;
        let mut every_source_answered = true;
        let mut any_source = false;
        for lookup in lookups {
            any_source = true;
            match lookup {
                Lookup::Found { confirmed } => {
                    found = Some(found.unwrap_or(false) || confirmed);
                }
                Lookup::NotFound => {}
                Lookup::Unanswered => every_source_answered = false,
            }
        }
        match found {
            Some(confirmed) => Presence::Found { confirmed },
            None if any_source && every_source_answered => Presence::Absent,
            None => Presence::Unknown,
        }
    }
}

/// Asks every source about `tracked` and combines their answers.
pub async fn look_up(lookups: &[Arc<dyn TxLookup>], tracked: &TrackedTx) -> Presence {
    let mut answers = Vec::with_capacity(lookups.len());
    for source in lookups {
        let answer = source.lookup(tracked).await;
        tracing::debug!(source = source.name(), txid = %tracked.txid, ?answer, "tx lookup");
        answers.push(answer);
    }
    Presence::combine(answers)
}

/// How long a transaction must stay absent before it is taken as gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbsenceWindow {
    /// Consecutive `Absent` checks needed.
    pub checks: u32,
    /// Minimum time between the first and the last of them.
    pub span: Duration,
}

impl AbsenceWindow {
    /// 3 checks over at least 10 minutes: far beyond mempool propagation and Electrum indexing
    /// lag (seconds), short enough that a dropped bundle's coins and claim come back the same
    /// session.
    pub const DEFAULT: AbsenceWindow = AbsenceWindow {
        checks: 3,
        span: Duration::from_secs(600),
    };

    /// [`Self::DEFAULT`], overridable with `SETTLE_ABSENCE_CHECKS` / `SETTLE_ABSENCE_WINDOW_SECS`
    /// (regtest tests shorten it).
    pub fn from_env() -> Self {
        let parse = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<u64>().ok());
        AbsenceWindow {
            checks: parse("SETTLE_ABSENCE_CHECKS")
                .and_then(|v| u32::try_from(v).ok())
                .filter(|v| *v > 0)
                .unwrap_or(Self::DEFAULT.checks),
            span: parse("SETTLE_ABSENCE_WINDOW_SECS")
                .map(Duration::from_secs)
                .unwrap_or(Self::DEFAULT.span),
        }
    }
}

/// The settle rule's verdict on one transaction after one check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Live: some source holds it.
    Found { confirmed: bool },
    /// Absent across the whole window: it is not, and will not be, on the network.
    Gone,
    /// Not decided yet.
    Open,
}

/// Counts consecutive `Absent` checks per transaction.
#[derive(Debug)]
pub struct AbsenceTracker {
    window: AbsenceWindow,
    /// txid → (consecutive absent checks, time of the first of them).
    absent: HashMap<Txid, (u32, Instant)>,
}

impl AbsenceTracker {
    pub fn new(window: AbsenceWindow) -> Self {
        Self {
            window,
            absent: HashMap::new(),
        }
    }

    /// Applies one check's [`Presence`] of `txid`, observed at `now`.
    pub fn observe(&mut self, txid: Txid, presence: Presence, now: Instant) -> Verdict {
        match presence {
            Presence::Found { confirmed } => {
                self.absent.remove(&txid);
                Verdict::Found { confirmed }
            }
            Presence::Unknown => {
                self.absent.remove(&txid);
                Verdict::Open
            }
            Presence::Absent => {
                let (checks, since) = self.absent.entry(txid).or_insert((0, now));
                *checks += 1;
                let spanned = now.saturating_duration_since(*since) >= self.window.span;
                if *checks >= self.window.checks && spanned {
                    Verdict::Gone
                } else {
                    Verdict::Open
                }
            }
        }
    }

    /// Stops counting `txid` (it was settled).
    pub fn forget(&mut self, txid: &Txid) {
        self.absent.remove(txid);
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Test double: answers per txid from a table that tests may change between checks;
    /// `fallback` for any txid not in it.
    pub struct StubLookup {
        answers: Mutex<HashMap<Txid, Lookup>>,
        fallback: Lookup,
    }

    impl StubLookup {
        pub fn always(fallback: Lookup) -> Self {
            Self {
                answers: Mutex::new(HashMap::new()),
                fallback,
            }
        }

        pub fn answer(&self, txid: Txid, lookup: Lookup) {
            self.answers.lock().unwrap().insert(txid, lookup);
        }
    }

    #[async_trait]
    impl TxLookup for StubLookup {
        fn name(&self) -> &'static str {
            "stub"
        }
        async fn lookup(&self, tracked: &TrackedTx) -> Lookup {
            self.answers
                .lock()
                .unwrap()
                .get(&tracked.txid)
                .copied()
                .unwrap_or(self.fallback)
        }
    }

    const FOUND: Lookup = Lookup::Found { confirmed: false };
    const MINED: Lookup = Lookup::Found { confirmed: true };

    /// #516: found anywhere wins; absent only when every source answered that it does not hold
    /// the tx — one source that did not answer keeps it unknown.
    #[test]
    fn answers_combine_across_sources() {
        use Lookup::{NotFound, Unanswered};
        let cases: Vec<(Vec<Lookup>, Presence)> = vec![
            (vec![NotFound, FOUND], Presence::Found { confirmed: false }),
            (
                vec![Unanswered, FOUND],
                Presence::Found { confirmed: false },
            ),
            (vec![FOUND, MINED], Presence::Found { confirmed: true }),
            (vec![NotFound, NotFound], Presence::Absent),
            (vec![NotFound, Unanswered], Presence::Unknown),
            (vec![Unanswered, Unanswered], Presence::Unknown),
            (vec![], Presence::Unknown),
        ];
        for (answers, expected) in cases {
            assert_eq!(Presence::combine(answers.clone()), expected, "{answers:?}");
        }
    }

    fn txid(n: u8) -> Txid {
        use bitcoin::hashes::Hash;
        Txid::from_byte_array([n; 32])
    }

    /// #516: gone only after `checks` consecutive absent checks spanning `span`; an unknown or
    /// found check in between starts the count again.
    #[test]
    fn a_tx_is_gone_only_after_the_whole_absence_window() {
        use Presence::{Absent, Unknown};
        let window = AbsenceWindow {
            checks: 3,
            span: Duration::from_secs(600),
        };
        let found = Presence::Found { confirmed: false };
        let minutes = |m: u64| Duration::from_secs(60 * m);
        // (checks as (minute, presence), verdict of the last one)
        let cases: Vec<(Vec<(u64, Presence)>, Verdict)> = vec![
            (vec![(0, Absent), (5, Absent), (10, Absent)], Verdict::Gone),
            // Enough checks, not enough time.
            (vec![(0, Absent), (1, Absent), (2, Absent)], Verdict::Open),
            // Enough time, not enough checks.
            (vec![(0, Absent), (10, Absent)], Verdict::Open),
            // An unanswered check resets the count.
            (
                vec![(0, Absent), (5, Unknown), (10, Absent), (15, Absent)],
                Verdict::Open,
            ),
            (
                vec![
                    (0, Absent),
                    (5, found),
                    (10, Absent),
                    (15, Absent),
                    (20, Absent),
                ],
                Verdict::Gone,
            ),
            (
                vec![(0, Absent), (5, Absent), (10, found)],
                Verdict::Found { confirmed: false },
            ),
        ];
        for (checks, expected) in cases {
            let start = Instant::now();
            let mut tracker = AbsenceTracker::new(window);
            let verdict = checks
                .iter()
                .map(|(minute, presence)| {
                    tracker.observe(txid(1), *presence, start + minutes(*minute))
                })
                .last()
                .expect("at least one check");
            assert_eq!(verdict, expected, "{checks:?}");
        }
    }
}
