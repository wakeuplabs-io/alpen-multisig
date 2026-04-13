//! Hardware wallet Tauri commands.

use crate::state::AppState;
use desktop_app::infrastructure::hw_wallet::{trezor, HwWalletInfo};
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
pub async fn sign_with_trezor(
    _state: State<'_, AppState>,
    sighash_hex: String,
    derivation_path: String,
) -> Result<SignatureResult, String> {
    trezor::sign_message(&sighash_hex, &derivation_path)
}
