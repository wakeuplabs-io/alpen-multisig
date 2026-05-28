//! Desktop broadcast configuration from process environment (never from the webview).

use bitcoin::{key::UntweakedKeypair, Network};
use strata_l1_txfmt::MagicBytes;

use crate::infrastructure::admin_wallet::AdminWalletError;

#[derive(Debug, thiserror::Error)]
pub enum BroadcastEnvError {
    #[error("missing required env var: {0}")]
    MissingEnv(&'static str),
    #[error("invalid Bitcoin network '{0}'; expected bitcoin/testnet/signet/regtest")]
    InvalidNetwork(String),
    #[error("invalid magic bytes hex: {0}")]
    InvalidMagicBytes(String),
    #[error("dev mnemonic signing is disabled (set ALLOW_DEV_MNEMONIC_SIGNING=1 for regtest)")]
    MnemonicSigningDisabled,
    #[error("admin wallet error: {0}")]
    AdminWallet(#[from] AdminWalletError),
}

impl From<BroadcastEnvError> for String {
    fn from(e: BroadcastEnvError) -> Self {
        e.to_string()
    }
}

/// Bitcoin + commit/reveal settings for commit/reveal broadcast (Tauri process only).
#[derive(Debug)]
pub struct BroadcastEnv {
    pub btc_rpc_url: String,
    pub btc_rpc_user: String,
    pub btc_rpc_pass: String,
    // Wired in Step 02-01 (commands/proposals.rs); suppress dead_code until then.
    #[allow(dead_code)]
    pub commit_reveal_keypair: UntweakedKeypair,
    /// Transitional alias for `commit_reveal_keypair`; call sites updated and field removed in Step 02-01.
    pub operator_keypair: UntweakedKeypair,
    pub magic_bytes: MagicBytes,
    pub asm_rpc_url: String,
    pub network: Network,
    pub confirm_poll_interval_ms: u64,
    pub confirm_timeout_ms: u64,
}

pub fn load_broadcast_env() -> Result<BroadcastEnv, BroadcastEnvError> {
    // Parse network first — needed for derive_commit_reveal_keypair
    let network_str = std::env::var("BITCOIN_NETWORK").unwrap_or_else(|_| "regtest".to_string());
    let network = parse_network(&network_str)?;

    // Read mnemonic
    let mnemonic = std::env::var("ADMIN_WALLET_REGTEST_MNEMONIC")
        .map_err(|_| BroadcastEnvError::MissingEnv("ADMIN_WALLET_REGTEST_MNEMONIC"))?;

    // Enforce dev mnemonic signing guard
    let allow = std::env::var("ALLOW_DEV_MNEMONIC_SIGNING")
        .map(|v| v == "1")
        .unwrap_or(false);
    if !allow {
        return Err(BroadcastEnvError::MnemonicSigningDisabled);
    }

    // Derive the commit/reveal keypair from the Admin Wallet seed
    let commit_reveal_keypair =
        crate::infrastructure::admin_wallet::commit_reveal_key::derive_commit_reveal_keypair(
            &mnemonic, network,
        )?;

    // Parse remaining env vars
    let btc_rpc_url = std::env::var("BITCOIN_RPC_URL")
        .map_err(|_| BroadcastEnvError::MissingEnv("BITCOIN_RPC_URL"))?;
    let btc_rpc_user = std::env::var("BITCOIN_RPC_USER")
        .map_err(|_| BroadcastEnvError::MissingEnv("BITCOIN_RPC_USER"))?;
    let btc_rpc_pass = std::env::var("BITCOIN_RPC_PASS")
        .map_err(|_| BroadcastEnvError::MissingEnv("BITCOIN_RPC_PASS"))?;
    let asm_rpc_url = std::env::var("STRATA_ADMIN_STATE_RPC_URL")
        .or_else(|_| std::env::var("ASM_RPC_URL"))
        .map_err(|_| BroadcastEnvError::MissingEnv("STRATA_ADMIN_STATE_RPC_URL"))?;
    let magic_hex =
        std::env::var("BITCOIN_MAGIC_BYTES_HEX").unwrap_or_else(|_| "414c504e".to_string());
    let confirm_poll_interval_ms = std::env::var("BROADCAST_CONFIRM_POLL_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5_000);
    let confirm_timeout_ms = std::env::var("BROADCAST_CONFIRM_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600_000);

    #[allow(deprecated)]
    Ok(BroadcastEnv {
        btc_rpc_url,
        btc_rpc_user,
        btc_rpc_pass,
        operator_keypair: commit_reveal_keypair,
        commit_reveal_keypair,
        magic_bytes: parse_magic_bytes(&magic_hex)?,
        asm_rpc_url,
        network,
        confirm_poll_interval_ms,
        confirm_timeout_ms,
    })
}

fn parse_network(network: &str) -> Result<Network, BroadcastEnvError> {
    match network {
        "bitcoin" => Ok(Network::Bitcoin),
        "testnet" => Ok(Network::Testnet),
        "signet" => Ok(Network::Signet),
        "regtest" => Ok(Network::Regtest),
        other => Err(BroadcastEnvError::InvalidNetwork(other.to_string())),
    }
}

fn parse_magic_bytes(hex_str: &str) -> Result<MagicBytes, BroadcastEnvError> {
    let bytes =
        hex::decode(hex_str).map_err(|e| BroadcastEnvError::InvalidMagicBytes(e.to_string()))?;
    let arr: [u8; 4] = bytes
        .try_into()
        .map_err(|_| BroadcastEnvError::InvalidMagicBytes("must be exactly 4 bytes".to_string()))?;
    Ok(MagicBytes::new(arr))
}

/// Shared mutex for serializing all env-var-manipulating tests across modules.
/// Import this in any test module that touches process environment.
#[cfg(test)]
pub(crate) static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

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

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn load_broadcast_env_happy_path_returns_broadcast_env() {
        let _g = ENV_TEST_LOCK.lock().unwrap();
        let vars = [
            ("BITCOIN_RPC_URL", Some("http://127.0.0.1:18443")),
            ("BITCOIN_RPC_USER", Some("user")),
            ("BITCOIN_RPC_PASS", Some("pass")),
            ("STRATA_ADMIN_STATE_RPC_URL", Some("http://127.0.0.1:9000")),
            ("ADMIN_WALLET_REGTEST_MNEMONIC", Some(TEST_MNEMONIC)),
            ("ALLOW_DEV_MNEMONIC_SIGNING", Some("1")),
            ("BITCOIN_NETWORK", Some("regtest")),
        ];
        for (k, v) in &vars {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
        let result = load_broadcast_env();
        for (k, _) in &vars {
            std::env::remove_var(k);
        }
        assert!(result.is_ok(), "expected Ok but got: {:?}", result.err());
    }

    #[test]
    fn load_broadcast_env_missing_mnemonic_returns_missing_env_error() {
        let _g = ENV_TEST_LOCK.lock().unwrap();
        std::env::set_var("BITCOIN_RPC_URL", "http://127.0.0.1:18443");
        std::env::set_var("BITCOIN_RPC_USER", "user");
        std::env::set_var("BITCOIN_RPC_PASS", "pass");
        std::env::set_var("STRATA_ADMIN_STATE_RPC_URL", "http://127.0.0.1:9000");
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        std::env::remove_var("ADMIN_WALLET_REGTEST_MNEMONIC");
        std::env::remove_var("BITCOIN_NETWORK");

        let result = load_broadcast_env();

        for k in &[
            "BITCOIN_RPC_URL",
            "BITCOIN_RPC_USER",
            "BITCOIN_RPC_PASS",
            "STRATA_ADMIN_STATE_RPC_URL",
            "ALLOW_DEV_MNEMONIC_SIGNING",
        ] {
            std::env::remove_var(k);
        }

        assert!(
            matches!(
                result,
                Err(BroadcastEnvError::MissingEnv(
                    "ADMIN_WALLET_REGTEST_MNEMONIC"
                ))
            ),
            "expected MissingEnv, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_missing_dev_guard_returns_mnemonic_signing_disabled() {
        let _g = ENV_TEST_LOCK.lock().unwrap();
        std::env::set_var("ADMIN_WALLET_REGTEST_MNEMONIC", TEST_MNEMONIC);
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");
        std::env::remove_var("BITCOIN_NETWORK");

        let result = load_broadcast_env();

        std::env::remove_var("ADMIN_WALLET_REGTEST_MNEMONIC");

        assert!(
            matches!(result, Err(BroadcastEnvError::MnemonicSigningDisabled)),
            "expected MnemonicSigningDisabled, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_dev_guard_false_returns_mnemonic_signing_disabled() {
        let _g = ENV_TEST_LOCK.lock().unwrap();
        std::env::set_var("ADMIN_WALLET_REGTEST_MNEMONIC", TEST_MNEMONIC);
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "0");
        std::env::remove_var("BITCOIN_NETWORK");

        let result = load_broadcast_env();

        std::env::remove_var("ADMIN_WALLET_REGTEST_MNEMONIC");
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");

        assert!(
            matches!(result, Err(BroadcastEnvError::MnemonicSigningDisabled)),
            "expected MnemonicSigningDisabled, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_invalid_mnemonic_propagates_admin_wallet_error() {
        let _g = ENV_TEST_LOCK.lock().unwrap();
        std::env::set_var(
            "ADMIN_WALLET_REGTEST_MNEMONIC",
            "not a valid mnemonic at all xyz",
        );
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        std::env::remove_var("BITCOIN_NETWORK");

        let result = load_broadcast_env();

        std::env::remove_var("ADMIN_WALLET_REGTEST_MNEMONIC");
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");

        assert!(
            matches!(result, Err(BroadcastEnvError::AdminWallet(_))),
            "expected AdminWallet error, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_invalid_magic_bytes_returns_invalid_magic_bytes_error() {
        let _g = ENV_TEST_LOCK.lock().unwrap();
        std::env::set_var("BITCOIN_RPC_URL", "http://127.0.0.1:18443");
        std::env::set_var("BITCOIN_RPC_USER", "user");
        std::env::set_var("BITCOIN_RPC_PASS", "pass");
        std::env::set_var("STRATA_ADMIN_STATE_RPC_URL", "http://127.0.0.1:9000");
        std::env::set_var("ADMIN_WALLET_REGTEST_MNEMONIC", TEST_MNEMONIC);
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        std::env::set_var("BITCOIN_NETWORK", "regtest");
        std::env::set_var("BITCOIN_MAGIC_BYTES_HEX", "aabbcc"); // 3 bytes, not 4

        let result = load_broadcast_env();

        for k in &[
            "BITCOIN_RPC_URL",
            "BITCOIN_RPC_USER",
            "BITCOIN_RPC_PASS",
            "STRATA_ADMIN_STATE_RPC_URL",
            "ADMIN_WALLET_REGTEST_MNEMONIC",
            "ALLOW_DEV_MNEMONIC_SIGNING",
            "BITCOIN_NETWORK",
            "BITCOIN_MAGIC_BYTES_HEX",
        ] {
            std::env::remove_var(k);
        }

        assert!(
            matches!(result, Err(BroadcastEnvError::InvalidMagicBytes(_))),
            "expected InvalidMagicBytes, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_regression_adjacent_parsing_preserved() {
        let _g = ENV_TEST_LOCK.lock().unwrap();
        std::env::set_var("BITCOIN_RPC_URL", "http://127.0.0.1:18443");
        std::env::set_var("BITCOIN_RPC_USER", "user");
        std::env::set_var("BITCOIN_RPC_PASS", "pass");
        std::env::set_var("STRATA_ADMIN_STATE_RPC_URL", "http://127.0.0.1:9000");
        std::env::set_var("ADMIN_WALLET_REGTEST_MNEMONIC", TEST_MNEMONIC);
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        std::env::set_var("BITCOIN_NETWORK", "signet");
        std::env::set_var("BITCOIN_MAGIC_BYTES_HEX", "deadbeef");
        std::env::set_var("BROADCAST_CONFIRM_POLL_MS", "1234");
        std::env::set_var("BROADCAST_CONFIRM_TIMEOUT_MS", "56789");

        let result = load_broadcast_env();

        for k in &[
            "BITCOIN_RPC_URL",
            "BITCOIN_RPC_USER",
            "BITCOIN_RPC_PASS",
            "STRATA_ADMIN_STATE_RPC_URL",
            "ADMIN_WALLET_REGTEST_MNEMONIC",
            "ALLOW_DEV_MNEMONIC_SIGNING",
            "BITCOIN_NETWORK",
            "BITCOIN_MAGIC_BYTES_HEX",
            "BROADCAST_CONFIRM_POLL_MS",
            "BROADCAST_CONFIRM_TIMEOUT_MS",
        ] {
            std::env::remove_var(k);
        }

        let env = result.expect("expected Ok with all vars set");
        assert_eq!(env.network, bitcoin::Network::Signet);
        assert_eq!(env.confirm_poll_interval_ms, 1234);
        assert_eq!(env.confirm_timeout_ms, 56789);
    }

    // with_env_var is retained for potential future use; suppress dead_code.
    #[allow(dead_code)]
    fn _with_env_var_used(key: &str, value: Option<&str>, f: impl FnOnce()) {
        with_env_var(key, value, f);
    }
}
