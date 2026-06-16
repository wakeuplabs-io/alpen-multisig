#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use tauri::Manager;

fn main() {
    desktop_app::infrastructure::env_loader::load_dotenv_files();
    let pending_reveals =
        desktop_app::infrastructure::pending_reveals_store::new_with_persistence();
    commands::invoke::attach_invoke_handlers(tauri::Builder::default())
        .manage(pending_reveals)
        .setup(|app| {
            use desktop_app::application::fee_estimation::FeeCacheState;
            use desktop_app::application::wallet_session::WalletSession;
            use desktop_app::infrastructure::node_config_store::{
                load_node_config, NodeConfigState,
            };
            use desktop_app::infrastructure::pending_reveals_store::init_app_data_dir;
            use std::sync::{Arc, RwLock};

            if let Ok(data_dir) = app.path().app_data_dir() {
                init_app_data_dir(data_dir);
            }

            let config = load_node_config(app.handle());
            let node_config_arc = Arc::new(RwLock::new(config));
            let wallet_session = WalletSession::with_node_config(Arc::clone(&node_config_arc));
            app.manage(NodeConfigState(node_config_arc));
            app.manage(wallet_session);
            // Fee cache (M2): survives across fee_rates_estimate calls; estimators
            // are rebuilt per call from the current node config.
            app.manage(FeeCacheState::default());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
