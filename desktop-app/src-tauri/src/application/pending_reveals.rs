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

/// Commit txid → reveal txid for all reveals still pending confirmation.
///
/// A commit in this map must never be RBF-bumped: replacing it changes its txid
/// and invalidates the pre-signed reveal that spends it (R1.0.1 — the ephemeral
/// envelope key is dropped right after signing, so the reveal cannot be re-signed).
/// Instead, its fee is bumped via CPFP on the reveal's change output, which is
/// why the reveal txid is carried alongside.
pub fn pending_commit_to_reveal(store: &PendingReveals) -> HashMap<String, String> {
    store
        .lock()
        .map(|map| {
            map.values()
                .map(|r| (r.commit_txid.clone(), r.reveal_txid.clone()))
                .collect()
        })
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
    fn pending_commit_to_reveal_maps_each_commit_to_its_reveal() {
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
        let map = pending_commit_to_reveal(&store);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("commit-1").map(String::as_str), Some("reveal-1"));
        assert_eq!(map.get("commit-2").map(String::as_str), Some("reveal-2"));
    }

    #[test]
    fn pending_commit_to_reveal_empty_store_returns_empty_map() {
        assert!(pending_commit_to_reveal(&new()).is_empty());
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
