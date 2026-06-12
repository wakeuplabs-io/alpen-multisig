use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::application::pending_reveals::{PendingReveal, PendingReveals};

const PENDING_REVEALS_FILE: &str = "pending-reveals.json";

static APP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init_app_data_dir(path: PathBuf) {
    let _ = APP_DATA_DIR.set(path);
}

fn get_app_data_dir() -> Option<PathBuf> {
    APP_DATA_DIR.get().cloned()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PendingRevealsStore {
    entries: HashMap<String, PendingReveal>,
}

pub fn new_with_persistence() -> PendingReveals {
    let store = load_pending_reveals();
    Arc::new(Mutex::new(store))
}

pub fn load_pending_reveals() -> HashMap<String, PendingReveal> {
    let data_dir = match get_app_data_dir() {
        Some(dir) => dir,
        None => return HashMap::new(),
    };
    let path = data_dir.join(PENDING_REVEALS_FILE);
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    let store: PendingRevealsStore = match serde_json::from_str(&contents) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    store.entries
}

pub fn save_pending_reveals(entries: &HashMap<String, PendingReveal>) {
    let data_dir = match get_app_data_dir() {
        Some(dir) => dir,
        None => return,
    };
    if std::fs::create_dir_all(&data_dir).is_err() {
        return;
    }
    let path = data_dir.join(PENDING_REVEALS_FILE);
    let store = PendingRevealsStore {
        entries: entries.clone(),
    };
    let Ok(json) = serde_json::to_string_pretty(&store) else {
        return;
    };
    let _ = std::fs::write(path, json);
}

pub fn insert_and_persist(store: &PendingReveals, action_id: String, reveal: PendingReveal) {
    if let Ok(mut guard) = store.lock() {
        guard.insert(action_id, reveal);
        save_pending_reveals(&guard);
    }
}

pub fn remove_and_persist(store: &PendingReveals, action_id: &str) {
    if let Ok(mut guard) = store.lock() {
        guard.remove(action_id);
        save_pending_reveals(&guard);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_pending_reveal() -> PendingReveal {
        PendingReveal {
            reveal_tx_hex: "aabbcc".to_string(),
            reveal_txid: "reveal-txid-1".to_string(),
            commit_txid: "commit-txid-1".to_string(),
        }
    }

    #[test]
    fn pending_reveals_store_serializes_and_deserializes() {
        let mut entries = HashMap::new();
        entries.insert("action-1".to_string(), test_pending_reveal());
        let store = PendingRevealsStore { entries };
        let json = serde_json::to_string(&store).expect("serialize");
        let loaded: PendingRevealsStore = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(loaded.entries.len(), 1);
        assert!(loaded.entries.contains_key("action-1"));
    }

    #[test]
    fn pending_reveals_store_empty_serializes() {
        let store = PendingRevealsStore::default();
        let json = serde_json::to_string(&store).expect("serialize");
        let loaded: PendingRevealsStore = serde_json::from_str(&json).expect("deserialize");
        assert!(loaded.entries.is_empty());
    }

    #[test]
    fn pending_reveals_store_handles_corrupted_json() {
        let result: Result<PendingRevealsStore, _> = serde_json::from_str("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn load_returns_empty_when_no_app_data_dir_set() {
        let result = load_pending_reveals();
        assert!(result.is_empty());
    }
}
