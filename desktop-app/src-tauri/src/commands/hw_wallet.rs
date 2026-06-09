//! Hardware wallet Tauri commands.

use desktop_app::infrastructure::hw_wallet::{ledger, trezor, HwWalletInfo};
use desktop_app::infrastructure::signing::{self, SignatureResult};

#[tauri::command]
pub async fn get_trezor_info(derivation_path: Option<String>) -> Result<HwWalletInfo, String> {
    trezor::connect(derivation_path)
}

#[tauri::command]
pub async fn verify_address_on_device(derivation_path: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || trezor::verify_address_on_device(derivation_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn sign_with_trezor(
    seqno: u64,
    action_hex: String,
    derivation_path: String,
) -> Result<SignatureResult, String> {
    let message = signing::render_signing_message(seqno, &action_hex)?;
    tokio::task::spawn_blocking(move || {
        trezor::sign_admin_sps65_binding(&message, &derivation_path)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn sign_challenge_with_trezor(
    challenge_hex: String,
    derivation_path: String,
) -> Result<SignatureResult, String> {
    tokio::task::spawn_blocking(move || {
        trezor::sign_admin_sps65_binding(&challenge_hex, &derivation_path)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_trezor_admin_wallet_xpub() -> Result<String, String> {
    let path = "m/86'/0'/73'".to_string();
    tokio::task::spawn_blocking(move || trezor::get_account_xpub(&path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_trezor_master_fingerprint() -> Result<u32, String> {
    tokio::task::spawn_blocking(trezor::get_master_fingerprint)
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
    use super::ledger_admin_wallet_xpub_path;

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
