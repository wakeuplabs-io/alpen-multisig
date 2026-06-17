//! Hardware wallet Tauri commands.

use bdk_wallet::bitcoin::Network;
use desktop_app::infrastructure::hw_wallet::hw_psbt_signer::HwDeviceType;
use desktop_app::infrastructure::hw_wallet::{ledger, trezor, AddressScriptType, HwWalletInfo};
use desktop_app::infrastructure::signing::{self, SignatureResult};

#[tauri::command]
pub async fn get_trezor_info(
    derivation_path: Option<String>,
    passphrase: Option<String>,
) -> Result<HwWalletInfo, String> {
    let pp = passphrase.unwrap_or_default();
    tokio::task::spawn_blocking(move || trezor::connect(derivation_path, &pp))
        .await
        .map_err(|e| e.to_string())?
}

/// Parses the device-kind IPC token into a [`HwDeviceType`] (case-insensitive).
fn parse_device_kind(token: &str) -> Result<HwDeviceType, String> {
    match token.trim().to_ascii_lowercase().as_str() {
        "trezor" => Ok(HwDeviceType::Trezor),
        "ledger" => Ok(HwDeviceType::Ledger),
        other => Err(format!(
            "unknown device type '{other}' (expected trezor or ledger)"
        )),
    }
}

/// Parses the network IPC token; defaults to regtest when absent (matches the session default).
fn parse_verify_network(network: Option<&str>) -> Network {
    match network.unwrap_or("regtest") {
        "testnet" => Network::Testnet,
        "bitcoin" | "mainnet" => Network::Bitcoin,
        "signet" => Network::Signet,
        _ => Network::Regtest,
    }
}

/// Confirms an address on the **connected** device screen (PRD §4.2 Admin ID,
/// §4.3.4.2 receive). Dispatches to Trezor or Ledger, with the script type
/// (`p2tr` receive / `p2wpkh` Admin ID) and the active network.
#[tauri::command]
pub async fn verify_address_on_device(
    derivation_path: String,
    device_type: String,
    script_type: String,
    network: Option<String>,
) -> Result<(), String> {
    let device = parse_device_kind(&device_type)?;
    let script = AddressScriptType::parse(&script_type)?;
    let net = parse_verify_network(network.as_deref());
    tokio::task::spawn_blocking(move || match device {
        HwDeviceType::Trezor => trezor::verify_address_on_device(derivation_path, script, net),
        HwDeviceType::Ledger => ledger::verify_address_on_device(derivation_path, script, net),
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn sign_with_trezor(
    seqno: u64,
    action_hex: String,
    derivation_path: String,
    passphrase: Option<String>,
) -> Result<SignatureResult, String> {
    let message = signing::render_signing_message(seqno, &action_hex)?;
    let pp = passphrase.unwrap_or_default();
    tokio::task::spawn_blocking(move || {
        trezor::sign_admin_sps65_binding(&message, &derivation_path, &pp)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn sign_challenge_with_trezor(
    challenge_hex: String,
    derivation_path: String,
    passphrase: Option<String>,
) -> Result<SignatureResult, String> {
    let pp = passphrase.unwrap_or_default();
    tokio::task::spawn_blocking(move || {
        trezor::sign_admin_sps65_binding(&challenge_hex, &derivation_path, &pp)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_trezor_admin_wallet_xpub(passphrase: Option<String>) -> Result<String, String> {
    let path = "m/86'/0'/73'".to_string();
    let pp = passphrase.unwrap_or_default();
    tokio::task::spawn_blocking(move || trezor::get_account_xpub(&path, &pp))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_trezor_master_fingerprint(passphrase: Option<String>) -> Result<u32, String> {
    let pp = passphrase.unwrap_or_default();
    tokio::task::spawn_blocking(move || trezor::get_master_fingerprint(&pp))
        .await
        .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------------------
// Ledger commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_ledger_info(derivation_path: Option<String>) -> Result<HwWalletInfo, String> {
    tokio::task::spawn_blocking(move || ledger::connect(derivation_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn sign_with_ledger(
    seqno: u64,
    action_hex: String,
    derivation_path: String,
) -> Result<SignatureResult, String> {
    let message = signing::render_signing_message(seqno, &action_hex)?;
    tokio::task::spawn_blocking(move || {
        ledger::sign_admin_sps65_binding(&message, &derivation_path)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// BIP-86 Admin Wallet account path for the Ledger, by network.
///
/// The Ledger Bitcoin **testnet** app only serves the testnet coin type (`1'`) and rejects
/// mainnet `0'` paths with APDU `6a82`. This mirrors the Ledger Admin ID convention
/// (`m/84'/1'/73'`). On mainnet the standard `0'` coin type is used.
fn ledger_admin_wallet_xpub_path(network: &str) -> &'static str {
    match network {
        "bitcoin" | "mainnet" => "m/86'/0'/73'",
        _ => "m/86'/1'/73'",
    }
}

#[tauri::command]
pub async fn get_ledger_admin_wallet_xpub() -> Result<String, String> {
    let network = std::env::var("BITCOIN_NETWORK").unwrap_or_else(|_| "regtest".to_string());
    let path = ledger_admin_wallet_xpub_path(&network).to_string();
    tokio::task::spawn_blocking(move || ledger::get_account_xpub(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_ledger_master_fingerprint() -> Result<u32, String> {
    eprintln!("[ledger] get_master_fingerprint called");
    let result = tokio::task::spawn_blocking(ledger::get_master_fingerprint)
        .await
        .map_err(|e| e.to_string())?;
    eprintln!("[ledger] get_master_fingerprint result: {:?}", result);
    result
}

#[tauri::command]
pub async fn sign_challenge_with_ledger(
    challenge_hex: String,
    derivation_path: String,
) -> Result<SignatureResult, String> {
    tokio::task::spawn_blocking(move || {
        ledger::sign_admin_sps65_binding(&challenge_hex, &derivation_path)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::{
        ledger_admin_wallet_xpub_path, parse_device_kind, parse_verify_network, AddressScriptType,
        HwDeviceType, Network,
    };

    #[test]
    fn parse_device_kind_accepts_both_devices_case_insensitively() {
        assert_eq!(parse_device_kind("trezor"), Ok(HwDeviceType::Trezor));
        assert_eq!(parse_device_kind("Ledger"), Ok(HwDeviceType::Ledger));
        assert!(parse_device_kind("keystone").is_err());
    }

    #[test]
    fn verify_dispatch_maps_script_type_per_authority() {
        // Receive row → P2TR; Admin ID row → P2WPKH.
        assert_eq!(
            AddressScriptType::parse("p2tr"),
            Ok(AddressScriptType::Taproot)
        );
        assert_eq!(
            AddressScriptType::parse("p2wpkh"),
            Ok(AddressScriptType::WitnessPubkeyHash)
        );
    }

    #[test]
    fn parse_verify_network_honors_active_network_and_defaults_to_regtest() {
        assert_eq!(parse_verify_network(None), Network::Regtest);
        assert_eq!(parse_verify_network(Some("testnet")), Network::Testnet);
        assert_eq!(parse_verify_network(Some("mainnet")), Network::Bitcoin);
        assert_eq!(parse_verify_network(Some("bitcoin")), Network::Bitcoin);
    }

    #[test]
    fn ledger_uses_testnet_coin_type_on_regtest() {
        // Regression: the Ledger testnet app rejects coin type 0' (APDU 6a82).
        // Regtest/testnet/signet must request coin type 1'.
        assert_eq!(ledger_admin_wallet_xpub_path("regtest"), "m/86'/1'/73'");
        assert_eq!(ledger_admin_wallet_xpub_path("testnet"), "m/86'/1'/73'");
        assert_eq!(ledger_admin_wallet_xpub_path("signet"), "m/86'/1'/73'");
    }

    #[test]
    fn ledger_uses_mainnet_coin_type_on_bitcoin() {
        assert_eq!(ledger_admin_wallet_xpub_path("bitcoin"), "m/86'/0'/73'");
        assert_eq!(ledger_admin_wallet_xpub_path("mainnet"), "m/86'/0'/73'");
    }
}
