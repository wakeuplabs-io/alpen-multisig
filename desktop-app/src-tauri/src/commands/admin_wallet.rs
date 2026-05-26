use desktop_app::infrastructure::admin_wallet::{get_external_address, load_admin_wallet};

#[derive(Debug, serde::Serialize)]
pub struct AdminWalletInfo {
    pub address: String,
    pub balance_sats: u64,
}

/// Inner logic — separated from the Tauri macro to allow direct unit testing.
pub fn admin_wallet_info_inner() -> Result<AdminWalletInfo, String> {
    let commit_funding =
        std::env::var("COMMIT_FUNDING").unwrap_or_else(|_| "bitcoind".to_string());
    if commit_funding != "admin_wallet" {
        return Err(format!(
            "COMMIT_FUNDING is '{}', expected 'admin_wallet'",
            commit_funding
        ));
    }

    let mnemonic = std::env::var("ADMIN_WALLET_REGTEST_MNEMONIC")
        .map_err(|_| "missing env var: ADMIN_WALLET_REGTEST_MNEMONIC".to_string())?;

    let network_str =
        std::env::var("BITCOIN_NETWORK").unwrap_or_else(|_| "regtest".to_string());
    let network = match network_str.as_str() {
        "regtest" => bdk_wallet::bitcoin::Network::Regtest,
        "testnet" => bdk_wallet::bitcoin::Network::Testnet,
        "bitcoin" | "mainnet" => bdk_wallet::bitcoin::Network::Bitcoin,
        other => return Err(format!("unsupported BITCOIN_NETWORK: {}", other)),
    };

    let mut wallet =
        load_admin_wallet(&mnemonic, network).map_err(|e| e.to_string())?;

    // Sync via bdk_bitcoind_rpc when RPC vars are present; otherwise skip (offline / test).
    let rpc_url = std::env::var("BITCOIN_RPC_URL").ok();
    if let Some(url) = rpc_url {
        use bdk_bitcoind_rpc::Emitter;
        let rpc_user = std::env::var("BITCOIN_RPC_USER").unwrap_or_default();
        let rpc_pass = std::env::var("BITCOIN_RPC_PASS").unwrap_or_default();
        let rpc = bdk_bitcoind_rpc::bitcoincore_rpc::Client::new(
            &url,
            bdk_bitcoind_rpc::bitcoincore_rpc::Auth::UserPass(rpc_user, rpc_pass),
        )
        .map_err(|e| e.to_string())?;

        let mut emitter = Emitter::new(&rpc, wallet.latest_checkpoint(), 0);
        while let Some(event) = emitter.next_block().map_err(|e| e.to_string())? {
            wallet
                .apply_block_connected_to(
                    &event.block,
                    event.block_height(),
                    event.connected_to(),
                )
                .map_err(|e| e.to_string())?;
        }
    }

    let balance = wallet.balance();
    let balance_sats = balance.trusted_spendable().to_sat();
    let address = get_external_address(&wallet).to_string();

    Ok(AdminWalletInfo {
        address,
        balance_sats,
    })
}

#[tauri::command]
pub fn get_admin_wallet_info() -> Result<AdminWalletInfo, String> {
    admin_wallet_info_inner()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn returns_ok_with_non_empty_address_when_admin_wallet_mode_and_mnemonic_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("COMMIT_FUNDING", "admin_wallet");
        std::env::set_var("ADMIN_WALLET_REGTEST_MNEMONIC", TEST_MNEMONIC);
        std::env::set_var("BITCOIN_NETWORK", "regtest");
        std::env::remove_var("BITCOIN_RPC_URL");

        let result = admin_wallet_info_inner();

        std::env::remove_var("COMMIT_FUNDING");
        std::env::remove_var("ADMIN_WALLET_REGTEST_MNEMONIC");
        std::env::remove_var("BITCOIN_NETWORK");

        let info = result.expect("expected Ok");
        assert!(
            !info.address.is_empty(),
            "expected non-empty address"
        );
        assert!(
            info.address.starts_with("bcrt1p"),
            "expected P2TR regtest address but got: {}",
            info.address
        );
    }

    #[test]
    fn returns_err_when_commit_funding_is_not_admin_wallet() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("COMMIT_FUNDING", "bitcoind");

        let result = admin_wallet_info_inner();

        std::env::remove_var("COMMIT_FUNDING");

        assert!(result.is_err(), "expected Err when COMMIT_FUNDING != admin_wallet");
        let err = result.unwrap_err();
        assert!(
            err.contains("admin_wallet"),
            "error should mention admin_wallet but got: {}",
            err
        );
    }
}
