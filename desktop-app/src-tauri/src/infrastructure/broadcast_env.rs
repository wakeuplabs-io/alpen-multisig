//! Desktop broadcast configuration from process environment (never from the webview).

use bitcoin::{key::UntweakedKeypair, Network};
use strata_l1_txfmt::MagicBytes;

/// Bitcoin + operator settings for commit/reveal broadcast (Tauri process only).
pub struct BroadcastEnv {
    pub btc_rpc_url: String,
    pub btc_rpc_user: String,
    pub btc_rpc_pass: String,
    pub btc_wallet_name: Option<String>,
    pub operator_keypair: UntweakedKeypair,
    pub magic_bytes: MagicBytes,
    pub asm_rpc_url: String,
    pub network: Network,
    pub confirm_poll_interval_ms: u64,
    pub confirm_timeout_ms: u64,
}

pub fn load_broadcast_env() -> Result<BroadcastEnv, String> {
    let btc_rpc_url = std::env::var("BITCOIN_RPC_URL")
        .map_err(|_| "BITCOIN_RPC_URL must be set for broadcast".to_string())?;
    let btc_rpc_user = std::env::var("BITCOIN_RPC_USER")
        .map_err(|_| "BITCOIN_RPC_USER must be set for broadcast".to_string())?;
    let btc_rpc_pass = std::env::var("BITCOIN_RPC_PASS")
        .map_err(|_| "BITCOIN_RPC_PASS must be set for broadcast".to_string())?;
    let btc_wallet_name = std::env::var("BITCOIN_WALLET_NAME").ok();
    let asm_rpc_url = std::env::var("STRATA_ADMIN_STATE_RPC_URL")
        .or_else(|_| std::env::var("ASM_RPC_URL"))
        .map_err(|_| {
            "STRATA_ADMIN_STATE_RPC_URL (or ASM_RPC_URL) must be set for broadcast".to_string()
        })?;
    let operator_hex = std::env::var("OPERATOR_SECRET_KEY_HEX")
        .map_err(|_| "OPERATOR_SECRET_KEY_HEX must be set for broadcast".to_string())?;
    let magic_hex =
        std::env::var("BITCOIN_MAGIC_BYTES_HEX").unwrap_or_else(|_| "414c504e".to_string());
    let network =
        parse_network(&std::env::var("BITCOIN_NETWORK").unwrap_or_else(|_| "regtest".to_string()))?;
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
        btc_wallet_name,
        operator_keypair: parse_operator_keypair(&operator_hex)?,
        magic_bytes: parse_magic_bytes(&magic_hex)?,
        asm_rpc_url,
        network,
        confirm_poll_interval_ms,
        confirm_timeout_ms,
    })
}

fn parse_network(network: &str) -> Result<Network, String> {
    match network {
        "bitcoin" => Ok(Network::Bitcoin),
        "testnet" => Ok(Network::Testnet),
        "signet" => Ok(Network::Signet),
        "regtest" => Ok(Network::Regtest),
        other => Err(format!(
            "unknown BITCOIN_NETWORK '{other}'; expected bitcoin/testnet/signet/regtest"
        )),
    }
}

fn parse_operator_keypair(secret_key_hex: &str) -> Result<UntweakedKeypair, String> {
    let sk_bytes =
        hex::decode(secret_key_hex).map_err(|e| format!("invalid operator key hex: {e}"))?;
    let sk = bitcoin::secp256k1::SecretKey::from_slice(&sk_bytes)
        .map_err(|e| format!("invalid operator secret key: {e}"))?;
    Ok(UntweakedKeypair::from_secret_key(
        bitcoin::secp256k1::SECP256K1,
        &sk,
    ))
}

fn parse_magic_bytes(hex_str: &str) -> Result<MagicBytes, String> {
    let bytes = hex::decode(hex_str).map_err(|e| format!("invalid magic bytes hex: {e}"))?;
    let arr: [u8; 4] = bytes
        .try_into()
        .map_err(|_| "BITCOIN_MAGIC_BYTES_HEX must be exactly 4 bytes".to_string())?;
    Ok(MagicBytes::new(arr))
}
