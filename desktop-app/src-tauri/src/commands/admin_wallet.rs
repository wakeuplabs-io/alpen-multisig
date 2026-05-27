use bdk_wallet::KeychainKind;
use desktop_app::application::wallet_service::{
    AddressDto, BalanceDto, SyncStatusDto, UtxoDto, WalletService,
};
use std::sync::Arc;

#[derive(Debug, serde::Serialize)]
pub struct AdminWalletInfo {
    pub address: String,
    pub balance_sats: u64,
}

/// Read admin wallet info from the shared `WalletService` — single source of truth.
///
/// Triggers a sync (which applies `WalletService::check_enabled` guard),
/// then returns the confirmed balance and external address index 0.
/// Returns `Err(Disabled)` if any of `COMMIT_FUNDING=admin_wallet`,
/// `BITCOIN_NETWORK=regtest`, `ALLOW_DEV_MNEMONIC_SIGNING=1` is missing.
pub async fn admin_wallet_info(svc: &WalletService) -> Result<AdminWalletInfo, String> {
    svc.sync().await.map_err(serialize_wallet_error)?;
    let balance = svc.get_balance().await.map_err(serialize_wallet_error)?;
    let address = svc
        .list_addresses(KeychainKind::External, 0, 1)
        .await
        .map_err(serialize_wallet_error)?
        .into_iter()
        .next()
        .ok_or_else(|| "no external address derivable".to_string())?
        .address;
    Ok(AdminWalletInfo {
        address,
        balance_sats: balance.confirmed_sats,
    })
}

#[tauri::command]
pub async fn get_admin_wallet_info(
    wallet_service: tauri::State<'_, Arc<WalletService>>,
) -> Result<AdminWalletInfo, String> {
    admin_wallet_info(wallet_service.inner()).await
}

fn serialize_wallet_error<E: serde::Serialize + std::fmt::Debug>(e: E) -> String {
    serde_json::to_string(&e).unwrap_or_else(|_| format!("{:?}", e))
}

#[tauri::command]
pub async fn admin_wallet_get_balance(
    wallet_service: tauri::State<'_, Arc<WalletService>>,
) -> Result<BalanceDto, String> {
    wallet_service
        .get_balance()
        .await
        .map_err(serialize_wallet_error)
}

#[tauri::command]
pub async fn admin_wallet_list_utxos(
    wallet_service: tauri::State<'_, Arc<WalletService>>,
) -> Result<Vec<UtxoDto>, String> {
    wallet_service
        .list_utxos()
        .await
        .map_err(serialize_wallet_error)
}

#[tauri::command]
pub async fn admin_wallet_list_addresses(
    keychain: String,
    page_index: u32,
    page_size: u32,
    wallet_service: tauri::State<'_, Arc<WalletService>>,
) -> Result<Vec<AddressDto>, String> {
    let keychain_kind = match keychain.to_lowercase().as_str() {
        "internal" => KeychainKind::Internal,
        _ => KeychainKind::External,
    };
    wallet_service
        .list_addresses(keychain_kind, page_index, page_size)
        .await
        .map_err(serialize_wallet_error)
}

#[tauri::command]
pub async fn admin_wallet_sync(
    wallet_service: tauri::State<'_, Arc<WalletService>>,
) -> Result<SyncStatusDto, String> {
    wallet_service.sync().await.map_err(serialize_wallet_error)
}

#[tauri::command]
pub fn admin_wallet_sync_status(
    wallet_service: tauri::State<'_, Arc<WalletService>>,
) -> SyncStatusDto {
    wallet_service.sync_status()
}

#[cfg(test)]
mod tests {
    use super::*;
    use desktop_app::infrastructure::admin_wallet::load_admin_wallet;
    use tokio::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::const_new(());

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn build_test_service() -> WalletService {
        let wallet = load_admin_wallet(TEST_MNEMONIC, bdk_wallet::bitcoin::Network::Regtest)
            .expect("wallet creation must succeed");
        WalletService::new(wallet)
    }

    fn clear_guard_env_vars() {
        std::env::remove_var("COMMIT_FUNDING");
        std::env::remove_var("BITCOIN_NETWORK");
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");
    }

    /// REGRESSION (UTXOs:0 with balance>0 bug): `get_admin_wallet_info` and
    /// `WalletService.sync()` MUST share the same guard. Before this fix the two
    /// paths diverged — info accepted only `COMMIT_FUNDING=admin_wallet`, sync
    /// required all three env vars — causing balance to render while UTXOs/list
    /// silently returned `[]` when `ALLOW_DEV_MNEMONIC_SIGNING` was unset.
    #[tokio::test]
    async fn admin_wallet_info_rejects_when_allow_dev_mnemonic_signing_missing() {
        let _guard = ENV_LOCK.lock().await;
        std::env::set_var("COMMIT_FUNDING", "admin_wallet");
        std::env::set_var("BITCOIN_NETWORK", "regtest");
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");

        let svc = build_test_service();
        let result = admin_wallet_info(&svc).await;

        clear_guard_env_vars();
        assert!(
            result.is_err(),
            "admin_wallet_info must reject when ALLOW_DEV_MNEMONIC_SIGNING is absent \
             (same guard as WalletService.sync) — otherwise the broadcast screen shows \
             balance from one wallet and UTXOs from another. Got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn admin_wallet_info_rejects_when_bitcoin_network_not_regtest() {
        let _guard = ENV_LOCK.lock().await;
        std::env::set_var("COMMIT_FUNDING", "admin_wallet");
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        std::env::set_var("BITCOIN_NETWORK", "bitcoin");

        let svc = build_test_service();
        let result = admin_wallet_info(&svc).await;

        clear_guard_env_vars();
        assert!(
            result.is_err(),
            "admin_wallet_info must reject when BITCOIN_NETWORK != regtest. Got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn admin_wallet_info_rejects_when_commit_funding_is_not_admin_wallet() {
        let _guard = ENV_LOCK.lock().await;
        std::env::set_var("COMMIT_FUNDING", "bitcoind");
        std::env::set_var("BITCOIN_NETWORK", "regtest");
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");

        let svc = build_test_service();
        let result = admin_wallet_info(&svc).await;

        clear_guard_env_vars();
        assert!(
            result.is_err(),
            "admin_wallet_info must reject when COMMIT_FUNDING != admin_wallet. Got: {:?}",
            result
        );
    }

    /// Verifies all 6 admin-wallet IPC commands are importable from the module
    /// (compile-time check — Tauri commands cannot be invoked directly in unit tests).
    #[test]
    fn six_ipc_command_functions_are_importable() {
        let _ = Some(get_admin_wallet_info as fn(_) -> _);
        let _ = Some(admin_wallet_get_balance as fn(_) -> _);
        let _ = Some(admin_wallet_list_utxos as fn(_) -> _);
        let _ = Some(admin_wallet_list_addresses as fn(_, _, _, _) -> _);
        let _ = Some(admin_wallet_sync as fn(_) -> _);
        let _sync_status_ptr: fn(tauri::State<'_, Arc<WalletService>>) -> SyncStatusDto =
            admin_wallet_sync_status;
        let _ = _sync_status_ptr;
    }
}
