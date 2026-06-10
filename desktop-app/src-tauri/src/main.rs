#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use tauri::Manager;

fn main() {
    desktop_app::infrastructure::env_loader::load_dotenv_files();
    let pending_reveals = desktop_app::application::pending_reveals::new();
    commands::invoke::attach_invoke_handlers(tauri::Builder::default())
        .manage(pending_reveals)
        .setup(|app| {
            use desktop_app::application::fee_estimation::{
                FeeEstimationService, FeeEstimationServiceState,
            };
            use desktop_app::application::wallet_session::WalletSession;
            use desktop_app::infrastructure::bitcoin_rpc::HttpBitcoinRpcClient;
            use desktop_app::infrastructure::electrum_fee_estimator::ElectrumFeeEstimator;
            use desktop_app::infrastructure::node_config_store::{
                load_node_config, NodeConfigState,
            };
            use desktop_app::infrastructure::node_fee_estimator::NodeFeeEstimator;
            use std::sync::{Arc, RwLock};

            let config = load_node_config(app.handle());
            let node_config_arc = Arc::new(RwLock::new(config.clone()));

            // Build the fee estimation service once — its in-memory cache survives
            // across fee_rates_estimate calls within the session.
            let rpc = Arc::new(HttpBitcoinRpcClient::new(
                config.btc_rpc_url(),
                config.btc_rpc_user(),
                config.btc_rpc_pass(),
            ));
            let fee_service = Arc::new(FeeEstimationService::new(vec![
                Arc::new(NodeFeeEstimator::new(rpc)),
                Arc::new(ElectrumFeeEstimator::new(config.electrum_url())),
            ]));

            let wallet_session = WalletSession::with_node_config(Arc::clone(&node_config_arc));
            app.manage(NodeConfigState(node_config_arc));
            app.manage(wallet_session);
            app.manage(FeeEstimationServiceState(fee_service));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
