//! Hardware wallet Tauri commands.

use crate::state::AppState;
use desktop_app::infrastructure::hw_wallet::{trezor, HwAddressEntry, HwWalletInfo};
use desktop_app::signing::SignatureResult;
use tauri::State;

#[tauri::command]
pub async fn get_trezor_info(
    _state: State<'_, AppState>,
    derivation_path: Option<String>,
) -> Result<HwWalletInfo, String> {
    trezor::connect(derivation_path)
}

#[tauri::command]
pub async fn list_hw_addresses(
    _state: State<'_, AppState>,
    count: Option<u32>,
) -> Result<Vec<HwAddressEntry>, String> {
    let n = count.unwrap_or(20) as usize;
    tokio::task::spawn_blocking(move || trezor::list_addresses(n))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn verify_address_on_device(
    _state: State<'_, AppState>,
    derivation_path: String,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || trezor::verify_address_on_device(derivation_path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn sign_with_trezor(
    _state: State<'_, AppState>,
    sighash_hex: String,
    derivation_path: String,
) -> Result<SignatureResult, String> {
    tokio::task::spawn_blocking(move || {
        trezor::sign_admin_sps65_binding(&sighash_hex, &derivation_path)
    })
    .await
    .map_err(|e| e.to_string())?
}
