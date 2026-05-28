#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    desktop_app::infrastructure::env_loader::load_dotenv_files();
    let wallet_session = desktop_app::application::wallet_session::WalletSession::empty();
    commands::invoke::attach_invoke_handlers(tauri::Builder::default())
        .manage(wallet_session)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
