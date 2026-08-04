//! Hardware wallet Tauri commands — unified by vendor dispatch.

use bdk_wallet::bitcoin::Network;
use desktop_app::infrastructure::admin_wallet::wallet::admin_wallet_account_path;
use desktop_app::infrastructure::hw_wallet::hw_psbt_signer::HwDeviceType;
use desktop_app::infrastructure::hw_wallet::trezor;
use desktop_app::infrastructure::hw_wallet::{AddressScriptType, HwWalletInfo};
use desktop_app::infrastructure::network_env::network_from_env;
use desktop_app::infrastructure::signing::{self, SignatureResult};

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

#[tauri::command]
pub async fn hw_wallet_connect(
    vendor: String,
    derivation_path: Option<String>,
) -> Result<HwWalletInfo, String> {
    let device = parse_device_kind(&vendor)?;
    tokio::task::spawn_blocking(move || device.connect(derivation_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn hw_wallet_sign(
    vendor: String,
    seqno: u64,
    action_hex: String,
    derivation_path: String,
) -> Result<SignatureResult, String> {
    let device = parse_device_kind(&vendor)?;
    let message = signing::render_signing_message(seqno, &action_hex)?;
    tokio::task::spawn_blocking(move || device.sign_sps65(&message, &derivation_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn hw_wallet_sign_challenge(
    vendor: String,
    challenge_message: String,
    derivation_path: String,
) -> Result<SignatureResult, String> {
    let device = parse_device_kind(&vendor)?;
    tokio::task::spawn_blocking(move || device.sign_sps65(&challenge_message, &derivation_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn hw_wallet_get_xpub(vendor: String) -> Result<String, String> {
    let device = parse_device_kind(&vendor)?;
    let network = network_from_env().map_err(|e| e.to_string())?;
    let path = admin_wallet_account_path(device, network).to_string();
    tokio::task::spawn_blocking(move || device.get_account_xpub(&path, network))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn hw_wallet_get_fingerprint(vendor: String) -> Result<u32, String> {
    let device = parse_device_kind(&vendor)?;
    tokio::task::spawn_blocking(move || device.get_master_fingerprint())
        .await
        .map_err(|e| e.to_string())?
}

/// Forgets the Trezor device session, so the next operation starts a clean one and the
/// device asks for the passphrase again.
///
/// Called when the signer disconnects the wallet: a session left behind would let the next
/// person on this machine reach the hidden wallet without re-entering the passphrase. Ledger
/// has no equivalent — it holds no host-visible session.
#[tauri::command]
pub async fn hw_wallet_end_session() -> Result<(), String> {
    trezor::forget_session();
    Ok(())
}

/// Renders the address at `derivation_path` on the connected device and returns the
/// exact string it displayed, so the UI can compare it with the address it shows for
/// the same key and path (#412 — hardware signers cannot render a raw public key).
#[tauri::command]
pub async fn verify_address_on_device(
    derivation_path: String,
    device_type: String,
    script_type: String,
    network: Option<String>,
) -> Result<String, String> {
    let device = parse_device_kind(&device_type)?;
    let script = AddressScriptType::parse(&script_type)?;
    let net = parse_verify_network(network.as_deref());
    tokio::task::spawn_blocking(move || device.verify_address(derivation_path, script, net))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::{
        admin_wallet_account_path, parse_device_kind, parse_verify_network, AddressScriptType,
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
    fn admin_wallet_account_path_ledger_uses_testnet_coin_type_off_mainnet() {
        assert_eq!(
            admin_wallet_account_path(HwDeviceType::Ledger, Network::Regtest),
            "m/86'/1'/73'"
        );
        assert_eq!(
            admin_wallet_account_path(HwDeviceType::Ledger, Network::Testnet),
            "m/86'/1'/73'"
        );
        assert_eq!(
            admin_wallet_account_path(HwDeviceType::Ledger, Network::Signet),
            "m/86'/1'/73'"
        );
    }

    #[test]
    fn admin_wallet_account_path_uses_mainnet_coin_type_on_bitcoin() {
        assert_eq!(
            admin_wallet_account_path(HwDeviceType::Ledger, Network::Bitcoin),
            "m/86'/0'/73'"
        );
        assert_eq!(
            admin_wallet_account_path(HwDeviceType::Trezor, Network::Bitcoin),
            "m/86'/0'/73'"
        );
    }

    #[test]
    fn admin_wallet_account_path_trezor_always_uses_mainnet_coin_type() {
        // Trezor only accepts/derives the BIP-86 account at coin type 0' on every network.
        for net in [Network::Regtest, Network::Testnet, Network::Signet] {
            assert_eq!(
                admin_wallet_account_path(HwDeviceType::Trezor, net),
                "m/86'/0'/73'"
            );
        }
    }
}
