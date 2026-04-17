#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use std::sync::Mutex;

fn main() {
    let backend_url =
        std::env::var("BACKEND_URL").unwrap_or_else(|_| "http://127.0.0.1:3000/api/v1".to_string());

    tauri::Builder::default()
        .manage(state::AppState {
            session_token: Mutex::new(None),
            backend_url,
        })
        .invoke_handler(tauri::generate_handler![
            commands::proposals::list_proposals,
            commands::hw_wallet::get_trezor_info,
            commands::hw_wallet::list_hw_addresses,
            commands::hw_wallet::verify_address_on_device,
            commands::hw_wallet::sign_with_trezor,
            commands::signing::compute_sighash,
            commands::signing::verify_threshold,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
