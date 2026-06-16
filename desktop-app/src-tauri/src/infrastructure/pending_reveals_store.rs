use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
    if !path.exists() {
        return HashMap::new();
    }
    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to read pending reveals store; starting with empty store"
            );
            return HashMap::new();
        }
    };
    let store: PendingRevealsStore = match serde_json::from_str(&contents) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "pending reveals store is corrupted; starting with empty store"
            );
            return HashMap::new();
        }
    };
    store.entries
}

fn atomic_write(path: &Path, contents: &str) -> std::io::Result<()> {
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, contents)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

pub fn save_pending_reveals(entries: &HashMap<String, PendingReveal>) {
    let data_dir = match get_app_data_dir() {
        Some(dir) => dir,
        None => return,
    };
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        tracing::warn!(
            path = %data_dir.display(),
            error = %e,
            "failed to create app data dir for pending reveals store"
        );
        return;
    }
    let path = data_dir.join(PENDING_REVEALS_FILE);
    let store = PendingRevealsStore {
        entries: entries.clone(),
    };
    let json = match serde_json::to_string_pretty(&store) {
        Ok(json) => json,
        Err(e) => {
            tracing::warn!(error = %e, "failed to serialize pending reveals store");
            return;
        }
    };
    if let Err(e) = atomic_write(&path, &json) {
        tracing::warn!(
            path = %path.display(),
            error = %e,
            "failed to persist pending reveals store"
        );
    }
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

    #[test]
    fn atomic_write_persists_readable_file() {
        let dir =
            std::env::temp_dir().join(format!("pending-reveals-atomic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join(PENDING_REVEALS_FILE);
        let payload = r#"{"entries":{}}"#;

        atomic_write(&path, payload).expect("atomic write");
        let read = std::fs::read_to_string(&path).expect("read back");
        assert_eq!(read, payload);
        assert!(!path.with_extension("tmp").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
