//! Desktop broadcast configuration — RPC endpoints from NodeConfig, network/dev flags from env.

use bitcoin::{key::UntweakedKeypair, Network};
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
    #[error("dev mnemonic signing is disabled (set ALLOW_DEV_MNEMONIC_SIGNING=1 for regtest)")]
    MnemonicSigningDisabled,
    #[error("wallet session required — log in with Palabras (dev mnemonic) before broadcast")]
    WalletSessionRequired,
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

/// Loads broadcast config: RPC endpoints from [`NodeConfig`], network/dev flags from env.
pub fn load_broadcast_env(
    wallet_session: &WalletSession,
    node_config: &NodeConfig,
) -> Result<BroadcastEnv, BroadcastEnvError> {
    let network_str = std::env::var("BITCOIN_NETWORK").unwrap_or_else(|_| "regtest".to_string());
    let network = parse_network(&network_str)?;

    let commit_reveal_keypair = resolve_commit_reveal_keypair(wallet_session)?;

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

fn resolve_commit_reveal_keypair(
    wallet_session: &WalletSession,
) -> Result<UntweakedKeypair, BroadcastEnvError> {
    let allow = std::env::var("ALLOW_DEV_MNEMONIC_SIGNING")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !allow {
        return Err(BroadcastEnvError::MnemonicSigningDisabled);
    }
    wallet_session
        .commit_reveal_keypair()
        .ok_or(BroadcastEnvError::WalletSessionRequired)
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
    use crate::infrastructure::admin_wallet::commit_reveal_key::derive_commit_reveal_keypair;
    use crate::infrastructure::node_config_store::NodeConfig;
    use bdk_wallet::bitcoin::Network;
    use bitcoin::secp256k1::XOnlyPublicKey;

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
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        std::env::set_var("BITCOIN_NETWORK", "regtest");
    }

    fn clear_dev_env() {
        for k in [
            "ALLOW_DEV_MNEMONIC_SIGNING",
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
    fn load_broadcast_env_uses_session_commit_reveal_key() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        set_dev_env();
        let session = session_with_mnemonic(TEST_MNEMONIC);
        let env = load_broadcast_env(&session, &test_node_config()).expect("load broadcast env");
        clear_dev_env();

        let expected = derive_commit_reveal_keypair(TEST_MNEMONIC, Network::Regtest).unwrap();
        let (expected_xonly, _) = XOnlyPublicKey::from_keypair(&expected);
        let (actual_xonly, _) = XOnlyPublicKey::from_keypair(&env.commit_reveal_keypair);
        assert_eq!(expected_xonly, actual_xonly);
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
    fn load_broadcast_env_missing_dev_guard_returns_mnemonic_signing_disabled() {
        let _g = ENV_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");
        std::env::remove_var("BITCOIN_NETWORK");

        let session = WalletSession::empty();
        let result = load_broadcast_env(&session, &test_node_config());

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
        let result = load_broadcast_env(&session, &test_node_config());

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
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
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
}
