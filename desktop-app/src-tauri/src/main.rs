#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use tauri::Manager;


fn main() {
    desktop_app::infrastructure::env_loader::load_dotenv_files();
    let wallet_session = desktop_app::application::wallet_session::WalletSession::empty();
    let pending_reveals = desktop_app::application::pending_reveals::new();
    commands::invoke::attach_invoke_handlers(tauri::Builder::default())
        .manage(wallet_session)
        .manage(pending_reveals)
        .setup(|app| {
            use desktop_app::infrastructure::node_config_store::{
                load_node_config, NodeConfigState,
            };
            use std::sync::{Arc, RwLock};
            let config = load_node_config(app.handle());
            app.manage(NodeConfigState(Arc::new(RwLock::new(config))));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
