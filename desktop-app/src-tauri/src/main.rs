#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use tauri::Manager;

#[allow(dead_code)]
fn build_wallet_service(
) -> Option<std::sync::Arc<desktop_app::application::wallet_service::WalletService>> {
    use desktop_app::application::wallet_service::WalletService;
    use desktop_app::infrastructure::admin_wallet::load_admin_wallet;

    let mnemonic = std::env::var("ADMIN_WALLET_REGTEST_MNEMONIC").ok()?;
    let network_str = std::env::var("BITCOIN_NETWORK").unwrap_or_else(|_| "regtest".to_string());
    let network = match network_str.as_str() {
        "regtest" => bdk_wallet::bitcoin::Network::Regtest,
        "testnet" => bdk_wallet::bitcoin::Network::Testnet,
        "bitcoin" | "mainnet" => bdk_wallet::bitcoin::Network::Bitcoin,
        _ => bdk_wallet::bitcoin::Network::Regtest,
    };

    let wallet = load_admin_wallet(&mnemonic, network).ok()?;
    Some(std::sync::Arc::new(WalletService::new(wallet)))
}

fn main() {
    desktop_app::infrastructure::env_loader::load_dotenv_files();
    let wallet_session = desktop_app::application::wallet_session::WalletSession::empty();
    let pending_reveals = desktop_app::application::pending_reveals::new();
    commands::invoke::attach_invoke_handlers(tauri::Builder::default())
        .manage(wallet_session)
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
