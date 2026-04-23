#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
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
