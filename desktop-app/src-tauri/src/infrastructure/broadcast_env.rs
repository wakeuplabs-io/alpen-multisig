//! Desktop broadcast configuration — RPC endpoints from NodeConfig, network/dev flags from env.

use bitcoin::Network;
use strata_l1_txfmt::MagicBytes;

use crate::application::wallet_session::WalletSession;
use crate::infrastructure::admin_wallet::AdminWalletError;
use crate::infrastructure::node_config_store::NodeConfig;

#[derive(Debug, thiserror::Error)]
pub enum BroadcastEnvError {
    #[error("invalid Bitcoin network '{0}'; expected bitcoin/testnet/signet/regtest")]
    InvalidNetwork(String),
    #[error("invalid magic bytes hex: {0}")]
    InvalidMagicBytes(String),
    #[error(
        "admin wallet session required — disconnect and reconnect your wallet (Ledger, Trezor, or Mnemonic) before broadcast"
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

/// Loads broadcast config: RPC endpoints from [`NodeConfig`], network/dev flags from env.
pub fn load_broadcast_env(
    wallet_session: &WalletSession,
    node_config: &NodeConfig,
) -> Result<BroadcastEnv, BroadcastEnvError> {
    let network = crate::infrastructure::network_env::network_from_env()
        .map_err(|e| BroadcastEnvError::InvalidNetwork(e.0))?;

    // Gate 1: wallet session must be active
    if wallet_session.current().is_none() {
        return Err(BroadcastEnvError::WalletSessionRequired);
    }
    // Gate 2: session must be able to sign on this network (per-signer capability —
    // mnemonic signer = regtest/testnet only; hardware signer = any network).
    if !wallet_session.can_sign() {
        return Err(BroadcastEnvError::ReadOnly);
    }

    let btc_rpc_url = node_config.btc_rpc_url().to_string();
    let btc_rpc_user = node_config.btc_rpc_user().to_string();
    let btc_rpc_pass = node_config.btc_rpc_pass().to_string();
    let asm_rpc_url = node_config.strata_rpc_url().to_string();
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
    use crate::infrastructure::node_config_store::NodeConfig;
    use bdk_wallet::bitcoin::Network;

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

    fn set_dev_env() {
        std::env::set_var("BITCOIN_NETWORK", "regtest");
    }

    fn clear_dev_env() {
        for k in [
            "BITCOIN_NETWORK",
            "BITCOIN_MAGIC_BYTES_HEX",
            "BROADCAST_CONFIRM_POLL_MS",
            "BROADCAST_CONFIRM_TIMEOUT_MS",
        ] {
            std::env::remove_var(k);
        }
    }

    fn test_node_config() -> NodeConfig {
        NodeConfig::default()
    }

    #[test]
    fn load_broadcast_env_happy_path_returns_broadcast_env() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_dev_env();
        let session = session_with_mnemonic(TEST_MNEMONIC);
        let result = load_broadcast_env(&session, &test_node_config());
        clear_dev_env();
        assert!(result.is_ok(), "expected Ok but got: {:?}", result.err());
    }

    #[test]
    fn load_broadcast_env_without_session_returns_wallet_session_required() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_dev_env();
        let session = WalletSession::empty();
        let result = load_broadcast_env(&session, &test_node_config());
        clear_dev_env();

        assert!(
            matches!(result, Err(BroadcastEnvError::WalletSessionRequired)),
            "expected WalletSessionRequired, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_invalid_magic_bytes_returns_invalid_magic_bytes_error() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_dev_env();
        std::env::set_var("BITCOIN_MAGIC_BYTES_HEX", "aabbcc");

        let session = session_with_mnemonic(TEST_MNEMONIC);
        let result = load_broadcast_env(&session, &test_node_config());
        clear_dev_env();

        assert!(
            matches!(result, Err(BroadcastEnvError::InvalidMagicBytes(_))),
            "expected InvalidMagicBytes, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_regression_adjacent_parsing_preserved() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("BITCOIN_NETWORK", "signet");
        std::env::set_var("BITCOIN_MAGIC_BYTES_HEX", "deadbeef");
        std::env::set_var("BROADCAST_CONFIRM_POLL_MS", "1234");
        std::env::set_var("BROADCAST_CONFIRM_TIMEOUT_MS", "56789");

        let session = session_with_mnemonic(TEST_MNEMONIC);
        let result = load_broadcast_env(&session, &test_node_config());
        clear_dev_env();

        let env = result.expect("expected Ok with session + env");
        assert_eq!(env.network, bitcoin::Network::Signet);
        assert_eq!(env.confirm_poll_interval_ms, 1234);
        assert_eq!(env.confirm_timeout_ms, 56789);
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
        let xpub = derive_regtest_xpub(TEST_MNEMONIC);
        let session = session_with_xpub(&xpub);
        let result = load_broadcast_env(&session, &test_node_config());
        assert!(
            matches!(result, Err(BroadcastEnvError::ReadOnly)),
            "expected ReadOnly, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_no_session_returns_wallet_session_required() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let session = WalletSession::empty();
        let result = load_broadcast_env(&session, &test_node_config());
        assert!(
            matches!(result, Err(BroadcastEnvError::WalletSessionRequired)),
            "expected WalletSessionRequired, got: {:?}",
            result
        );
    }

    #[test]
    fn load_broadcast_env_mnemonic_session_passes_gates() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("BITCOIN_NETWORK", "regtest");
        let session = session_with_mnemonic(TEST_MNEMONIC);
        let result = load_broadcast_env(&session, &test_node_config());
        clear_dev_env();
        assert!(
            result.is_ok(),
            "expected Ok with mnemonic session, got: {:?}",
            result.err()
        );
    }
}
