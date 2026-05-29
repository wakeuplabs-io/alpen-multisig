//! Load `desktop-app/.env` at a fixed path so broadcast config does not depend on process CWD.

use std::path::{Path, PathBuf};

/// Canonical `.env` for the desktop app (`desktop-app/.env`).
pub fn dotenv_path() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().map(|parent| parent.join(".env"))
}

/// Load `desktop-app/.env`, then `.env` in the process CWD if different (IDE launches).
pub fn load_dotenv_files() {
    if let Some(path) = dotenv_path() {
        try_load_dotenv(&path);
    }
    let _ = dotenvy::dotenv();
}

fn try_load_dotenv(path: &Path) {
    if path.is_file() {
        let _ = dotenvy::from_path(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use the shared env lock from broadcast_env to prevent concurrent env-var races.
    use crate::infrastructure::broadcast_env::ENV_TEST_LOCK;

    fn restore_var(key: &str, previous: Option<String>) {
        match previous {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn dotenv_path_points_at_desktop_app_env() {
        let path = dotenv_path().expect("parent .env path");
        assert!(
            path.ends_with("desktop-app/.env"),
            "expected desktop-app/.env, got {}",
            path.display()
        );
    }

    #[test]
    fn dotenvy_does_not_overwrite_existing_vars() {
        let _guard = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir =
            std::env::temp_dir().join(format!("alpen-env-loader-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tempdir");
        let first = dir.join("first.env");
        let second = dir.join("second.env");
        std::fs::write(
            &first,
            "ALPEN_ENV_LOADER_FOO=first\nALPEN_ENV_LOADER_BAR=first\n",
        )
        .expect("write first");
        std::fs::write(&second, "ALPEN_ENV_LOADER_FOO=second\n").expect("write second");

        let foo_prev = std::env::var("ALPEN_ENV_LOADER_FOO").ok();
        let bar_prev = std::env::var("ALPEN_ENV_LOADER_BAR").ok();
        std::env::remove_var("ALPEN_ENV_LOADER_FOO");
        std::env::remove_var("ALPEN_ENV_LOADER_BAR");

        try_load_dotenv(&first);
        try_load_dotenv(&second);

        assert_eq!(std::env::var("ALPEN_ENV_LOADER_FOO").expect("foo"), "first");
        assert_eq!(std::env::var("ALPEN_ENV_LOADER_BAR").expect("bar"), "first");

        restore_var("ALPEN_ENV_LOADER_FOO", foo_prev);
        restore_var("ALPEN_ENV_LOADER_BAR", bar_prev);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Verifies `desktop-app/.env` (when present) supplies broadcast configuration.
    #[test]
    fn project_dotenv_supports_broadcast_env_load() {
        let _guard = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let path = match dotenv_path() {
            Some(p) if p.is_file() => p,
            _ => return,
        };

        const TEST_MNEMONIC: &str =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let keys = [
            "BITCOIN_RPC_URL",
            "BITCOIN_RPC_USER",
            "BITCOIN_RPC_PASS",
            "STRATA_ADMIN_STATE_RPC_URL",
            "ALLOW_DEV_MNEMONIC_SIGNING",
            "BITCOIN_NETWORK",
            "BITCOIN_MAGIC_BYTES_HEX",
        ];
        let saved: Vec<_> = keys.iter().map(|k| (*k, std::env::var(k).ok())).collect();
        for key in keys {
            std::env::remove_var(key);
        }

        try_load_dotenv(&path);
        if std::env::var("ALLOW_DEV_MNEMONIC_SIGNING").is_err() {
            for (key, prev) in saved {
                restore_var(key, prev);
            }
            return;
        }

        let session = crate::application::wallet_session::WalletSession::empty();
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(session.init_from_mnemonic(TEST_MNEMONIC, None, None))
            .expect("session init for broadcast env smoke test");
        let result = crate::infrastructure::broadcast_env::load_broadcast_env(&session);

        for (key, prev) in saved {
            restore_var(key, prev);
        }

        assert!(
            result.is_ok(),
            "load_broadcast_env failed after loading {}: {}",
            path.display(),
            result.err().map(|e| e.to_string()).unwrap_or_default()
        );
    }
}
