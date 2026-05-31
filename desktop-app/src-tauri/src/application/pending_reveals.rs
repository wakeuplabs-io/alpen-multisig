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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_returns_empty_store() {
        let store = new();
        assert_eq!(store.lock().unwrap().len(), 0);
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
