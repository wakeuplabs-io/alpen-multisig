//! Hardware wallet Tauri commands.

use crate::state::AppState;
use desktop_app::infrastructure::hw_wallet::{trezor, HwAddressEntry, HwWalletInfo};
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
