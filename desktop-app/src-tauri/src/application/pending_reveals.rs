use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct PendingReveal {
    pub reveal_tx_hex: String,
    pub reveal_txid: String,
    pub commit_txid: String,
}

pub type PendingReveals = Arc<Mutex<HashMap<String, PendingReveal>>>;

pub fn new() -> PendingReveals {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Commit txids of all reveals still pending confirmation.
///
/// A commit in this set must never be RBF-bumped: replacing it changes its txid
/// and invalidates the pre-signed reveal that spends it (R1.0.1 — the ephemeral
/// envelope key is dropped right after signing, so the reveal cannot be re-signed).
pub fn pending_commit_txids(store: &PendingReveals) -> std::collections::HashSet<String> {
    store
        .lock()
        .map(|map| map.values().map(|r| r.commit_txid.clone()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_returns_empty_store() {
        let store = new();
        assert_eq!(store.lock().unwrap().len(), 0);
    }

    #[test]
    fn pending_commit_txids_collects_all_commit_ids() {
        let store = new();
        for i in 1..=2 {
            store.lock().unwrap().insert(
                format!("action-{i}"),
                PendingReveal {
                    reveal_tx_hex: "aabbcc".to_string(),
                    reveal_txid: format!("reveal-{i}"),
                    commit_txid: format!("commit-{i}"),
                },
            );
        }
        let txids = pending_commit_txids(&store);
        assert_eq!(txids.len(), 2);
        assert!(txids.contains("commit-1"));
        assert!(txids.contains("commit-2"));
    }

    #[test]
    fn pending_commit_txids_empty_store_returns_empty_set() {
        assert!(pending_commit_txids(&new()).is_empty());
    }

    #[test]
    fn insert_and_retrieve() {
        let store = new();
        store.lock().unwrap().insert(
            "action-1".to_string(),
            PendingReveal {
                reveal_tx_hex: "aabbcc".to_string(),
                reveal_txid: "txid-1".to_string(),
                commit_txid: "commit-1".to_string(),
            },
        );
        let guard = store.lock().unwrap();
        let entry = guard.get("action-1").expect("should be present");
        assert_eq!(entry.reveal_tx_hex, "aabbcc");
    }
}
