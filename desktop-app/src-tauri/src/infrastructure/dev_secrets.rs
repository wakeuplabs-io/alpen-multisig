//! Dev-only secret material over IPC (Wave 2 Decision #2).
//!
//! Production / release builds must not register or serve mnemonic/raw-key signing commands
//! unless `ALLOW_DEV_MNEMONIC_SIGNING` is set. Debug builds enable the dev surface by default.

const ALLOW_DEV_MNEMONIC_SIGNING_ENV: &str = "ALLOW_DEV_MNEMONIC_SIGNING";

/// Whether mnemonic / raw-key signing IPC is available (handler registration + invoke).
pub fn dev_mnemonic_signing_ipc_enabled() -> bool {
    if cfg!(debug_assertions) {
        return true;
    }
    dev_mnemonic_signing_env_flag()
}

/// Gate for commands that accept BIP39 mnemonics or raw secret key hex from the webview.
pub fn ensure_dev_mnemonic_signing_allowed() -> Result<(), String> {
    if dev_mnemonic_signing_ipc_enabled() {
        Ok(())
    } else {
        Err(
            "mnemonic and raw secret-key signing over IPC are disabled in production builds; \
             set ALLOW_DEV_MNEMONIC_SIGNING=1 for dev/E2E, or use a debug build"
                .to_string(),
        )
    }
}

fn dev_mnemonic_signing_env_flag() -> bool {
    std::env::var(ALLOW_DEV_MNEMONIC_SIGNING_ENV)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn with_env_var(key: &str, value: Option<&str>, f: impl FnOnce()) {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        let previous = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        f();
        match previous {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn env_flag_off_when_unset() {
        with_env_var(ALLOW_DEV_MNEMONIC_SIGNING_ENV, None, || {
            assert!(!dev_mnemonic_signing_env_flag());
        });
    }

    #[test]
    fn env_flag_on_for_one_or_true() {
        with_env_var(ALLOW_DEV_MNEMONIC_SIGNING_ENV, Some("1"), || {
            assert!(dev_mnemonic_signing_env_flag());
        });
        with_env_var(ALLOW_DEV_MNEMONIC_SIGNING_ENV, Some("true"), || {
            assert!(dev_mnemonic_signing_env_flag());
        });
    }

    #[test]
    fn ensure_blocks_when_release_profile_and_no_env() {
        if cfg!(debug_assertions) {
            return;
        }
        with_env_var(ALLOW_DEV_MNEMONIC_SIGNING_ENV, None, || {
            assert!(ensure_dev_mnemonic_signing_allowed().is_err());
        });
    }

    #[test]
    fn ensure_allows_when_env_set_in_release_profile() {
        if cfg!(debug_assertions) {
            return;
        }
        with_env_var(ALLOW_DEV_MNEMONIC_SIGNING_ENV, Some("1"), || {
            ensure_dev_mnemonic_signing_allowed().unwrap();
        });
    }
}
