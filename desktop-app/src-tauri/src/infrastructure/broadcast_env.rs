//! Desktop broadcast configuration from process environment (never from the webview).

use bitcoin::Network;
use strata_l1_txfmt::MagicBytes;

use crate::application::wallet_session::WalletSession;
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
    #[error(
        "admin wallet session required — disconnect and reconnect your wallet (Ledger, Trezor, or Palabras) before broadcast"
    )]
    WalletSessionRequired,
    #[error("admin wallet is watch-only; hardware wallet required to sign")]
    ReadOnly,
    #[error("admin wallet error: {0}")]
    AdminWallet(#[from] AdminWalletError),
}

impl From<BroadcastEnvError> for String {
    fn from(e: BroadcastEnvError) -> Self {
        e.to_string()
    }
}

/// Bitcoin + RPC/network settings for commit/reveal broadcast (Tauri process only).
#[derive(Debug)]
pub struct BroadcastEnv {
    pub btc_rpc_url: String,
    pub btc_rpc_user: String,
    pub btc_rpc_pass: String,
    pub magic_bytes: MagicBytes,
    pub asm_rpc_url: String,
    pub network: Network,
    pub confirm_poll_interval_ms: u64,
    pub confirm_timeout_ms: u64,
}

/// Loads RPC/asm broadcast config from env and enforces signing gates against
/// [`WalletSession`].
pub fn load_broadcast_env(
    wallet_session: &WalletSession,
) -> Result<BroadcastEnv, BroadcastEnvError> {
    let network_str = std::env::var("BITCOIN_NETWORK").unwrap_or_else(|_| "regtest".to_string());
    let network = parse_network(&network_str)?;

    // Gate 1: dev mnemonic signing must be explicitly enabled
    let allow = std::env::var("ALLOW_DEV_MNEMONIC_SIGNING")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !allow {
        return Err(BroadcastEnvError::MnemonicSigningDisabled);
    }
    // Gate 2: wallet session must be active
    if wallet_session.current().is_none() {
        return Err(BroadcastEnvError::WalletSessionRequired);
    }
    // Gate 3: session must not be read-only
    if !wallet_session.can_sign() {
        return Err(BroadcastEnvError::ReadOnly);
    }

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

    Ok(BroadcastEnv {
        btc_rpc_url,
        btc_rpc_user,
        btc_rpc_pass,
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
    use crate::application::wallet_session::WalletSession;
    use bdk_wallet::bitcoin::Network;

    fn with_env_var(key: &str, value: Option<&str>, f: impl FnOnce()) {
        let _guard = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

    fn session_with_mnemonic(mnemonic: &str) -> WalletSession {
        let session = WalletSession::empty();
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(session.init_from_mnemonic(mnemonic, None, None))
            .expect("session init");
        session
    }

    fn set_broadcast_rpc_env() {
        std::env::set_var("BITCOIN_RPC_URL", "http://127.0.0.1:18443");
        std::env::set_var("BITCOIN_RPC_USER", "user");
        std::env::set_var("BITCOIN_RPC_PASS", "pass");
        std::env::set_var("STRATA_ADMIN_STATE_RPC_URL", "http://127.0.0.1:9000");
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        std::env::set_var("BITCOIN_NETWORK", "regtest");
    }

    fn clear_broadcast_rpc_env() {
        for k in [
            "BITCOIN_RPC_URL",
            "BITCOIN_RPC_USER",
            "BITCOIN_RPC_PASS",
            "STRATA_ADMIN_STATE_RPC_URL",
            "ALLOW_DEV_MNEMONIC_SIGNING",
            "BITCOIN_NETWORK",
            "BITCOIN_MAGIC_BYTES_HEX",
            "BROADCAST_CONFIRM_POLL_MS",
            "BROADCAST_CONFIRM_TIMEOUT_MS",
        ] {
            std::env::remove_var(k);
        }
    }

    #[test]
    fn load_broadcast_env_happy_path_returns_broadcast_env() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_broadcast_rpc_env();
        let session = session_with_mnemonic(TEST_MNEMONIC);
        let result = load_broadcast_env(&session);
        clear_broadcast_rpc_env();
        assert!(result.is_ok(), "expected Ok but got: {:?}", result.err());
    }

    #[test]
    fn load_broadcast_env_without_session_returns_wallet_session_required() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_broadcast_rpc_env();
        let session = WalletSession::empty();
        let result = load_broadcast_env(&session);
        clear_broadcast_rpc_env();

        assert!(
            matches!(result, Err(BroadcastEnvError::WalletSessionRequired)),
            "expected WalletSessionRequired, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_missing_dev_guard_returns_mnemonic_signing_disabled() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");
        std::env::remove_var("BITCOIN_NETWORK");

        let session = WalletSession::empty();
        let result = load_broadcast_env(&session);

        assert!(
            matches!(result, Err(BroadcastEnvError::MnemonicSigningDisabled)),
            "expected MnemonicSigningDisabled, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_dev_guard_false_returns_mnemonic_signing_disabled() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "0");
        std::env::remove_var("BITCOIN_NETWORK");

        let session = WalletSession::empty();
        let result = load_broadcast_env(&session);

        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");

        assert!(
            matches!(result, Err(BroadcastEnvError::MnemonicSigningDisabled)),
            "expected MnemonicSigningDisabled, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_invalid_magic_bytes_returns_invalid_magic_bytes_error() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_broadcast_rpc_env();
        std::env::set_var("BITCOIN_MAGIC_BYTES_HEX", "aabbcc");

        let session = session_with_mnemonic(TEST_MNEMONIC);
        let result = load_broadcast_env(&session);
        clear_broadcast_rpc_env();

        assert!(
            matches!(result, Err(BroadcastEnvError::InvalidMagicBytes(_))),
            "expected InvalidMagicBytes, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_regression_adjacent_parsing_preserved() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("BITCOIN_RPC_URL", "http://127.0.0.1:18443");
        std::env::set_var("BITCOIN_RPC_USER", "user");
        std::env::set_var("BITCOIN_RPC_PASS", "pass");
        std::env::set_var("STRATA_ADMIN_STATE_RPC_URL", "http://127.0.0.1:9000");
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        std::env::set_var("BITCOIN_NETWORK", "signet");
        std::env::set_var("BITCOIN_MAGIC_BYTES_HEX", "deadbeef");
        std::env::set_var("BROADCAST_CONFIRM_POLL_MS", "1234");
        std::env::set_var("BROADCAST_CONFIRM_TIMEOUT_MS", "56789");

        let session = session_with_mnemonic(TEST_MNEMONIC);
        let result = load_broadcast_env(&session);
        clear_broadcast_rpc_env();

        let env = result.expect("expected Ok with session + RPC env");
        assert_eq!(env.network, bitcoin::Network::Signet);
        assert_eq!(env.confirm_poll_interval_ms, 1234);
        assert_eq!(env.confirm_timeout_ms, 56789);
    }

    // with_env_var is retained for potential future use; suppress dead_code.
    #[allow(dead_code)]
    fn _with_env_var_used(key: &str, value: Option<&str>, f: impl FnOnce()) {
        with_env_var(key, value, f);
    }

    fn derive_regtest_xpub(mnemonic_str: &str) -> String {
        use bdk_wallet::bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
        use bdk_wallet::bitcoin::secp256k1::Secp256k1;
        use std::str::FromStr;
        let mnemonic = bip39::Mnemonic::parse(mnemonic_str).unwrap();
        let seed = mnemonic.to_seed("");
        let secp = Secp256k1::new();
        let xpriv = Xpriv::new_master(Network::Regtest, &seed).unwrap();
        let path = DerivationPath::from_str("m/86h/0h/73h").unwrap();
        let account_xpriv = xpriv.derive_priv(&secp, &path).unwrap();
        Xpub::from_priv(&secp, &account_xpriv).to_string()
    }

    fn session_with_xpub(xpub: &str) -> WalletSession {
        let session = WalletSession::empty();
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(session.init_from_xpub(xpub, None))
            .expect("session init");
        session
    }

    #[test]
    fn load_broadcast_env_watch_only_session_returns_read_only() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        let xpub = derive_regtest_xpub(TEST_MNEMONIC);
        let session = session_with_xpub(&xpub);
        let result = load_broadcast_env(&session);
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");
        assert!(
            matches!(result, Err(BroadcastEnvError::ReadOnly)),
            "expected ReadOnly, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_no_session_returns_wallet_session_required() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        let session = WalletSession::empty();
        let result = load_broadcast_env(&session);
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");
        assert!(
            matches!(result, Err(BroadcastEnvError::WalletSessionRequired)),
            "expected WalletSessionRequired, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_mnemonic_session_passes_gates() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("BITCOIN_RPC_URL", "http://127.0.0.1:18443");
        std::env::set_var("BITCOIN_RPC_USER", "user");
        std::env::set_var("BITCOIN_RPC_PASS", "pass");
        std::env::set_var("STRATA_ADMIN_STATE_RPC_URL", "http://127.0.0.1:9000");
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        std::env::set_var("BITCOIN_NETWORK", "regtest");
        let session = session_with_mnemonic(TEST_MNEMONIC);
        let result = load_broadcast_env(&session);
        clear_broadcast_rpc_env();
        assert!(
            result.is_ok(),
            "expected Ok with mnemonic session, got: {:?}",
            result.err()
        );
    }
}
