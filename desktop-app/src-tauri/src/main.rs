#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

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

    let wallet_service = build_wallet_service().unwrap_or_else(|| {
        use desktop_app::application::wallet_service::WalletService;
        use desktop_app::infrastructure::admin_wallet::load_admin_wallet;
        // Fallback: construct with a dummy wallet so managed state is always present.
        // All methods will return Disabled unless env vars are correctly configured.
        const DUMMY_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(DUMMY_MNEMONIC, bdk_wallet::bitcoin::Network::Regtest)
            .expect("fallback wallet creation must succeed");
        std::sync::Arc::new(WalletService::new(wallet))
    });

    commands::invoke::attach_invoke_handlers(tauri::Builder::default())
        .manage(wallet_service)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
