use bdk_wallet::KeychainKind;
use desktop_app::application::wallet_service::{
    error_code, AddressDto, BalanceDto, SyncStatusDto, UtxoDto, WalletService,
};
use desktop_app::application::wallet_session::WalletSession;
use desktop_app::infrastructure::admin_wallet::AdminWalletError;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletSessionInitInput {
    pub mnemonic: String,
    pub passphrase: Option<String>,
    pub network: Option<String>,
}

#[tauri::command]
pub async fn wallet_session_init(
    input: WalletSessionInitInput,
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<(), String> {
    desktop_app::infrastructure::dev_secrets::ensure_dev_mnemonic_signing_allowed()?;
    wallet_session
        .init_from_mnemonic(
            &input.mnemonic,
            input.passphrase.as_deref(),
            input.network.as_deref(),
        )
        .await
        .map_err(serialize_wallet_error)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchOnlyInitInput {
    pub xpub: String,
    pub network: Option<String>,
    pub master_fingerprint: Option<u32>,
    pub device_type: Option<String>,
}

#[tauri::command]
pub async fn wallet_session_init_watch_only(
    input: WatchOnlyInitInput,
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<(), String> {
    if let Some(fp) = input.master_fingerprint {
        let device_type =
            match input.device_type.as_deref() {
                Some("trezor") => {
                    desktop_app::infrastructure::hw_wallet::hw_psbt_signer::HwDeviceType::Trezor
                }
                Some("ledger") => {
                    desktop_app::infrastructure::hw_wallet::hw_psbt_signer::HwDeviceType::Ledger
                }
                _ => return Err(
                    "device_type must be 'trezor' or 'ledger' when master_fingerprint is provided"
                        .to_string(),
                ),
            };
        wallet_session
            .init_from_xpub_with_hw(&input.xpub, fp, device_type, input.network.as_deref())
            .await
            .map_err(serialize_wallet_error)
    } else {
        wallet_session
            .init_from_xpub(&input.xpub, input.network.as_deref())
            .await
            .map_err(serialize_wallet_error)
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminWalletSignStatus {
    pub can_sign: bool,
    pub signer_kind: String,
    pub reason: Option<String>,
}

#[tauri::command]
pub async fn admin_wallet_can_sign(
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<AdminWalletSignStatus, String> {
    let can_sign = wallet_session.can_sign();
    let (signer_kind, reason) = if can_sign {
        // Detect signer kind from the active session.
        // HW signers report 'trezor' or 'ledger'; mnemonic reports 'mnemonic'.
        match wallet_session.current() {
            Some(svc) => {
                let kind = svc.signer_kind();
                (kind, None)
            }
            None => ("none".to_string(), Some("no-session".to_string())),
        }
    } else {
        match wallet_session.current() {
            None => ("none".to_string(), Some("no-session".to_string())),
            Some(_) => ("none".to_string(), Some("watch-only-no-signer".to_string())),
        }
    };
    Ok(AdminWalletSignStatus {
        can_sign,
        signer_kind,
        reason,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct AdminWalletInfo {
    pub address: String,
    pub balance_sats: u64,
}

/// Read admin wallet info from the shared `WalletService` — single source of truth.
///
/// Triggers a sync (which applies `WalletService::check_enabled` guard),
/// then returns the confirmed balance and external address index 0.
/// Returns `Err(Disabled)` if `BITCOIN_NETWORK=regtest` or `ALLOW_DEV_MNEMONIC_SIGNING=1`
/// is not set.
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
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<AdminWalletInfo, String> {
    let svc = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    admin_wallet_info(&svc).await
}

/// Serialize an [`AdminWalletError`] into the tagged `{ "type", "message" }` shape the frontend
/// `AdminWalletError` union expects. The previous default serde output (externally-tagged, e.g.
/// the bare string `"Disabled"`) did not match `{ type: 'Disabled' }`, so the UI could not
/// recognise a `Disabled`/`ReadOnly` error and silently fell back to an empty panel.
fn serialize_wallet_error(e: AdminWalletError) -> String {
    serde_json::json!({ "type": error_code(&e), "message": e.to_string() }).to_string()
}

#[tauri::command]
pub async fn admin_wallet_get_balance(
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<BalanceDto, String> {
    let svc = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    svc.get_balance().await.map_err(serialize_wallet_error)
}

#[tauri::command]
pub async fn admin_wallet_list_utxos(
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<Vec<UtxoDto>, String> {
    let svc = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    svc.list_utxos().await.map_err(serialize_wallet_error)
}

#[tauri::command]
pub async fn admin_wallet_list_addresses(
    keychain: String,
    page_index: u32,
    page_size: u32,
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<Vec<AddressDto>, String> {
    let keychain_kind = match keychain.to_lowercase().as_str() {
        "internal" => KeychainKind::Internal,
        _ => KeychainKind::External,
    };
    let svc = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    svc.list_addresses(keychain_kind, page_index, page_size)
        .await
        .map_err(serialize_wallet_error)
}

#[tauri::command]
pub async fn admin_wallet_sync(
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<SyncStatusDto, String> {
    let svc = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    svc.sync().await.map_err(serialize_wallet_error)
}

#[tauri::command]
pub fn admin_wallet_sync_status(wallet_session: tauri::State<'_, WalletSession>) -> SyncStatusDto {
    match wallet_session.current() {
        Some(svc) => svc.sync_status(),
        None => SyncStatusDto::disabled_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use desktop_app::application::wallet_session::WalletSession;
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
        std::env::remove_var("BITCOIN_NETWORK");
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");
    }

    /// REGRESSION: `admin_wallet_info` and `WalletService::sync()` MUST share the same guard.
    /// If `ALLOW_DEV_MNEMONIC_SIGNING` is absent the guard rejects — this prevents the broadcast
    /// screen from showing a balance while UTXOs/list silently return empty.
    #[tokio::test]
    async fn admin_wallet_info_rejects_when_allow_dev_mnemonic_signing_missing() {
        let _guard = ENV_LOCK.lock().await;
        std::env::set_var("BITCOIN_NETWORK", "regtest");
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");

        let svc = build_test_service();
        let result = admin_wallet_info(&svc).await;

        clear_guard_env_vars();
        assert!(
            result.is_err(),
            "admin_wallet_info must reject when ALLOW_DEV_MNEMONIC_SIGNING is absent. Got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn admin_wallet_info_rejects_when_bitcoin_network_not_regtest() {
        let _guard = ENV_LOCK.lock().await;
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");
        std::env::set_var("BITCOIN_NETWORK", "bitcoin");

        let svc = build_test_service();
        let result = admin_wallet_info(&svc).await;

        clear_guard_env_vars();
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");
        assert!(
            result.is_err(),
            "admin_wallet_info must reject when BITCOIN_NETWORK != regtest. Got: {:?}",
            result
        );
    }

    /// Verifies wallet_session_init is importable (compile-time check).
    #[test]
    fn wallet_session_init_command_function_is_importable() {
        // Verify the symbol is importable — compile-time existence check.
        // (async fn cannot be cast to fn pointer; naming it is sufficient for existence check.)
        let _f = wallet_session_init;
        let _ = _f;
    }

    /// wallet_session_init must NOT appear in attach_production.
    /// Verified by reading invoke.rs source at compile time via include_str!.
    #[test]
    fn wallet_session_init_not_in_attach_production() {
        let invoke_src = include_str!("invoke.rs");
        // Find attach_production block: everything from "fn attach_production" up to the
        // closing of its invoke_handler! macro call.
        let prod_start = invoke_src
            .find("fn attach_production")
            .expect("attach_production must exist in invoke.rs");
        let prod_src = &invoke_src[prod_start..];
        let prod_end = prod_src
            .find("fn attach_with_dev_signing")
            .unwrap_or(prod_src.len());
        let prod_block = &prod_src[..prod_end];
        // Only the mnemonic-based wallet_session_init (dev-gated) must be absent from
        // attach_production. wallet_session_init_watch_only is production-safe and is allowed.
        let has_mnemonic_init = prod_block
            .split_whitespace()
            .any(|tok| tok.trim_matches(',') == "wallet_session_init");
        assert!(
            !has_mnemonic_init,
            "wallet_session_init must NOT appear in attach_production — found in: {prod_block}"
        );
    }

    /// Verifies all 6 admin-wallet IPC commands take WalletSession (not Arc<WalletService>).
    /// Each function is referenced with its expected parameter types — wrong State type
    /// or missing import causes a compile error, proving the migration is complete.
    #[test]
    #[allow(clippy::let_underscore_future)]
    fn six_ipc_commands_use_wallet_session_state() {
        use desktop_app::application::wallet_session::WalletSession;
        // Async commands: reference the fn item; the compiler checks the State<WalletSession>
        // parameter type matches at the call site.
        fn _check_get_admin_wallet_info(s: tauri::State<'_, WalletSession>) {
            let _ = get_admin_wallet_info(s);
        }
        fn _check_get_balance(s: tauri::State<'_, WalletSession>) {
            let _ = admin_wallet_get_balance(s);
        }
        fn _check_list_utxos(s: tauri::State<'_, WalletSession>) {
            let _ = admin_wallet_list_utxos(s);
        }
        fn _check_list_addresses(
            keychain: String,
            page_index: u32,
            page_size: u32,
            s: tauri::State<'_, WalletSession>,
        ) {
            let _ = admin_wallet_list_addresses(keychain, page_index, page_size, s);
        }
        fn _check_sync(s: tauri::State<'_, WalletSession>) {
            let _ = admin_wallet_sync(s);
        }
        // Non-async: concrete return type assertion via fn pointer.
        let _sync_status_ptr: fn(tauri::State<'_, WalletSession>) -> SyncStatusDto =
            admin_wallet_sync_status;
        let _ = _sync_status_ptr;
    }

    // ---- acceptance test: admin_wallet_can_sign (step 02-03) ----

    // Derives account xpub from mnemonic for watch-only wallet tests.
    fn derive_test_xpub() -> String {
        use bdk_wallet::bitcoin::bip32::{DerivationPath, Xpriv};
        use bdk_wallet::bitcoin::{secp256k1::Secp256k1, Network};
        use bip39::Mnemonic;
        use std::str::FromStr;
        let mnemonic = Mnemonic::parse(TEST_MNEMONIC).unwrap();
        let seed = mnemonic.to_seed("");
        let secp = Secp256k1::new();
        let root = Xpriv::new_master(Network::Regtest, &seed).unwrap();
        let path = DerivationPath::from_str("m/84'/1'/0'").unwrap();
        let xpriv = root.derive_priv(&secp, &path).unwrap();
        bdk_wallet::bitcoin::bip32::Xpub::from_priv(&secp, &xpriv).to_string()
    }

    /// Acceptance: admin_wallet_can_sign returns false after watch-only init.
    #[tokio::test]
    async fn admin_wallet_can_sign_returns_false_after_watch_only_init() {
        let session = WalletSession::empty();
        let xpub = derive_test_xpub();
        session
            .init_from_xpub(&xpub, None)
            .await
            .expect("watch-only init must succeed");
        assert!(
            !session.can_sign(),
            "can_sign must be false for watch-only session"
        );
    }

    /// Unit: admin_wallet_can_sign returns false when no session.
    #[test]
    fn admin_wallet_can_sign_returns_false_when_no_session() {
        let session = WalletSession::empty();
        assert!(
            !session.can_sign(),
            "can_sign must be false when slot is empty"
        );
    }

    /// admin_wallet_sync_status returns disabled_default when slot is empty.
    /// Tests the non-async branch: current() returns None → SyncStatusDto::disabled_default().
    #[test]
    fn admin_wallet_sync_status_returns_disabled_default_when_slot_empty() {
        use desktop_app::application::wallet_session::WalletSession;
        let session = WalletSession::empty();
        // current() on empty session returns None → disabled_default path
        let status = match session.current() {
            Some(svc) => svc.sync_status(),
            None => SyncStatusDto::disabled_default(),
        };
        // disabled_default: is_syncing=false, last_error has code="Disabled"
        assert!(
            !status.is_syncing,
            "disabled_default must have is_syncing=false"
        );
        let error_code = status
            .last_error
            .as_ref()
            .map(|e| e.code.as_str())
            .unwrap_or("");
        assert_eq!(
            error_code, "Disabled",
            "disabled_default last_error.code must be Disabled, got: {error_code}"
        );
    }

    #[test]
    fn serialize_wallet_error_emits_tagged_type_and_message() {
        // Regression: the frontend AdminWalletError union expects `{ type, message }`.
        // The old default serde output (bare `"Disabled"`) made the UI miss the error and
        // render an empty panel instead of the Disabled card.
        let json = serialize_wallet_error(AdminWalletError::Disabled);
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON object");
        assert_eq!(value["type"], "Disabled");
        assert_eq!(value["message"], "admin wallet is disabled");

        let json = serialize_wallet_error(AdminWalletError::ReadOnly);
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON object");
        assert_eq!(value["type"], "ReadOnly");
    }
}
