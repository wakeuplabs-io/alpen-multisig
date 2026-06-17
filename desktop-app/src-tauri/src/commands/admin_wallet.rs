use bdk_wallet::KeychainKind;
use desktop_app::application::pending_reveals::{pending_commit_to_reveal, PendingReveals};
use desktop_app::application::tx_broadcaster::TxBroadcaster;
use desktop_app::application::wallet_send::{
    send_error_code, SendAddressValidationDto, SendError, SendEstimateDto, SendInput, SendResultDto,
};
use desktop_app::application::wallet_service::{
    error_code, AddressDto, BalanceDto, SyncStatusDto, UtxoDto, WalletService,
};
use desktop_app::application::wallet_session::WalletSession;
use desktop_app::application::wallet_transactions::{
    bump_error_code, BumpFeeError, BumpFeeResultDto, UnconfirmedTxDto,
};
use desktop_app::domain::fee_rate::{FeeRate, FALLBACK_MIN_RELAY_SAT_PER_KVB};
use desktop_app::infrastructure::admin_wallet::AdminWalletError;
use desktop_app::infrastructure::bitcoin_rpc::{BitcoinRpcClient, HttpBitcoinRpcClient};
use desktop_app::infrastructure::electrum_broadcaster::ElectrumBroadcaster;
use desktop_app::infrastructure::node_broadcaster::NodeBroadcaster;
use desktop_app::infrastructure::node_config_store::NodeConfigState;

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
    /// Specific connected device (`trezor` | `ledger`) for HW sessions; `None` otherwise.
    /// Drives verify-on-device dispatch (PRD §4.2 / §4.3.4.2) and the Send confirm-on-device UI.
    pub device_type: Option<String>,
    /// Active session network token (`regtest` | `testnet` | `signet` | `bitcoin`), so the
    /// verify-on-device call uses the right coin path.
    pub network: String,
}

/// Maps internal signer labels to the FE capability DTO (`hardware` | `mnemonic` | `none`).
fn capability_signer_kind(raw: &str) -> &'static str {
    match raw {
        "trezor" | "ledger" => "hardware",
        "mnemonic" => "mnemonic",
        _ => "none",
    }
}

/// The specific HW device token (`trezor` | `ledger`) for verify-on-device dispatch,
/// or `None` for software / no signer.
fn hw_device_type(raw: &str) -> Option<String> {
    match raw {
        "trezor" | "ledger" => Some(raw.to_string()),
        _ => None,
    }
}

/// Network token for the FE (`regtest` | `testnet` | `signet` | `bitcoin`).
fn network_token(network: bdk_wallet::bitcoin::Network) -> &'static str {
    use bdk_wallet::bitcoin::Network;
    match network {
        Network::Bitcoin => "bitcoin",
        Network::Testnet | Network::Testnet4 => "testnet",
        Network::Signet => "signet",
        Network::Regtest => "regtest",
    }
}

#[tauri::command]
pub async fn admin_wallet_can_sign(
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<AdminWalletSignStatus, String> {
    let can_sign = wallet_session.can_sign();
    let current = wallet_session.current();
    let network = current
        .as_ref()
        .map(|svc| network_token(svc.network()).to_string())
        .unwrap_or_else(|| "regtest".to_string());
    let raw_kind = current.as_ref().map(|svc| svc.signer_kind());
    let device_type = raw_kind.as_deref().and_then(hw_device_type);
    let (signer_kind, reason) = if can_sign {
        match raw_kind {
            Some(kind) => (capability_signer_kind(&kind).to_string(), None),
            None => ("none".to_string(), Some("no-session".to_string())),
        }
    } else {
        match current {
            None => ("none".to_string(), Some("no-session".to_string())),
            Some(_) => ("none".to_string(), Some("watch-only-no-signer".to_string())),
        }
    };
    Ok(AdminWalletSignStatus {
        can_sign,
        signer_kind,
        reason,
        device_type,
        network,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct AdminWalletInfo {
    pub address: String,
    pub balance_sats: u64,
}

/// Read admin wallet info from the shared `WalletService` — single source of truth.
///
/// Returns the cached state without triggering a sync (sync is handled separately by
/// `admin_wallet_sync`; calling it here blocks the UI on the indexer). Returns
/// `Err(Disabled)` if no wallet session is active.
///
/// - `address` is the current receive address (gap-aware `next_receive_address`, the same
///   rotating address the wallet panel shows) — not a fixed external index 0, which goes
///   stale once funds move to later indices or change.
/// - `balance_sats` is the TOTAL spendable balance (confirmed + unconfirmed): after a
///   spend the funds sit in change, which BDK can spend again, so the broadcast funding
///   gate must count it.
pub async fn admin_wallet_info(svc: &WalletService) -> Result<AdminWalletInfo, String> {
    let balance = svc.get_balance().await.map_err(serialize_wallet_error)?;
    let address = svc
        .next_receive_address()
        .await
        .map_err(serialize_wallet_error)?
        .address;
    Ok(AdminWalletInfo {
        address,
        balance_sats: balance.total_sats,
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

/// Returns the next unused external (receive) address (R1.3 — receive rotation).
///
/// Idempotent until the address is credited and observed during sync, after which it
/// rotates to the next unused index. Returns the tagged `Disabled` error if no session
/// is active.
#[tauri::command]
pub async fn admin_wallet_next_receive_address(
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<AddressDto, String> {
    let svc = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    svc.next_receive_address()
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

/// Serialize a [`BumpFeeError`] into the tagged `{ "type", "message" }` shape the
/// frontend `AdminWalletError` union expects (same convention as `serialize_wallet_error`).
fn serialize_bump_error(e: &BumpFeeError) -> String {
    serde_json::json!({ "type": bump_error_code(e), "message": e.to_string() }).to_string()
}

/// Lists unconfirmed transactions sent from the Admin Wallet (PRD §4.3.3).
///
/// Reads the last-synced wallet state (no chain round trip — the panel's
/// sync-then-read flow keeps it fresh). Governance commits with a pending
/// pre-signed reveal are flagged, offered CPFP as the bump method, and carry
/// the commit+reveal package fee/rate.
#[tauri::command]
pub async fn admin_wallet_list_unconfirmed_txs(
    wallet_session: tauri::State<'_, WalletSession>,
    pending: tauri::State<'_, PendingReveals>,
) -> Result<Vec<UnconfirmedTxDto>, String> {
    let svc = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    let commit_to_reveal = pending_commit_to_reveal(&pending);
    svc.list_unconfirmed_sent_txs(&commit_to_reveal)
        .await
        .map_err(serialize_wallet_error)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BumpFeeInput {
    pub txid: String,
    pub fee_rate_sat_per_kvb: u64,
}

/// Bumps the fee of an unconfirmed wallet transaction (PRD §4.3.3): RBF for plain
/// sends, CPFP (child on the reveal's change) for pending governance commits.
/// Validates the rate, syncs best-effort so the wallet view is fresh, then
/// builds/signs/broadcasts through `WalletService::bump_fee` (Electrum first,
/// node RPC fallback) and re-syncs so the panel converges.
#[tauri::command]
pub async fn admin_wallet_bump_fee(
    input: BumpFeeInput,
    wallet_session: tauri::State<'_, WalletSession>,
    node_config: tauri::State<'_, NodeConfigState>,
    pending: tauri::State<'_, PendingReveals>,
) -> Result<BumpFeeResultDto, String> {
    // Validate the rate before any wallet or network work.
    let rate = FeeRate::new(input.fee_rate_sat_per_kvb, FALLBACK_MIN_RELAY_SAT_PER_KVB)
        .map_err(|e| serialize_bump_error(&BumpFeeError::from(e)))?;

    let svc = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    let cfg = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .clone();
    let btc_rpc: std::sync::Arc<dyn BitcoinRpcClient> = std::sync::Arc::new(
        HttpBitcoinRpcClient::new(cfg.btc_rpc_url(), cfg.btc_rpc_user(), cfg.btc_rpc_pass()),
    );
    // Broadcaster chain: Electrum first, node fallback — same order as governance broadcast.
    let broadcasters: Vec<std::sync::Arc<dyn TxBroadcaster>> = vec![
        std::sync::Arc::new(ElectrumBroadcaster::new(cfg.electrum_url())),
        std::sync::Arc::new(NodeBroadcaster::new(std::sync::Arc::clone(&btc_rpc))),
    ];

    // Best-effort pre-sync: a stale view is ultimately caught by the node, which
    // rejects replacements of confirmed or already-replaced transactions.
    // F-008: Track sync failure to surface as warning in result.
    let sync_warning = match svc.sync().await {
        Ok(_) => None,
        Err(e) => {
            tracing::warn!(error = %e, "pre-bump sync failed; proceeding with last-known wallet state");
            Some(format!(
                "Wallet sync failed before bump: {}. Proceeding with last-known state. \
                 If the bump fails, sync the wallet and retry.",
                e
            ))
        }
    };

    let commit_to_reveal = pending_commit_to_reveal(&pending);
    let mut result = svc
        .bump_fee(&input.txid, rate, &commit_to_reveal, &broadcasters)
        .await
        .map_err(|e| serialize_bump_error(&e))?;

    // F-008: Attach sync warning to result for UI display.
    result.sync_warning = sync_warning;

    // Best-effort post-sync so the panel reflects the replacement immediately.
    if let Err(e) = svc.sync().await {
        tracing::warn!(error = %e, "post-bump sync failed; panel will converge on next sync");
    }

    Ok(result)
}

/// Serialize a [`SendError`] into the tagged `{ "type", "message" }` shape the
/// frontend `AdminWalletError` union expects (same convention as `serialize_wallet_error`).
fn serialize_send_error(e: &SendError) -> String {
    serde_json::json!({ "type": send_error_code(e), "message": e.to_string() }).to_string()
}

/// Validates a Send destination against the session network (Phase 6 P6.2,
/// PRD §4.3.5.1). Pure parse — no signer required (watch-only sessions can
/// type addresses), no wallet lock, no network I/O. Validation failures are a
/// **successful** result (form states, not faults); the frontend renders the
/// exact PRD copy from `reason` + `expectedNetwork`. Errs only when no wallet
/// session is active (tagged `Disabled`).
#[tauri::command]
pub async fn admin_wallet_validate_send_address(
    address: String,
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<SendAddressValidationDto, String> {
    let svc = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    Ok(svc.validate_send_address(&address))
}

/// Dry-run estimate for the Send form (Phase 6 P6.3, PRD §4.3.5.2/§4.3.5.3).
///
/// Validates the rate, then builds the requested transaction (and a drain
/// dry-run for the Max boundary) over the last-synced wallet state — never
/// signed, never broadcast, no pre-sync (it runs per debounced keystroke; the
/// panel's sync loop keeps state fresh). `InsufficientFunds` /
/// `AmountBelowDust` here are how the form learns the §4.3.5.2 boundary
/// before Confirm. No signer required — watch-only sessions may preview.
#[tauri::command]
pub async fn admin_wallet_estimate_send(
    input: SendInput,
    wallet_session: tauri::State<'_, WalletSession>,
) -> Result<SendEstimateDto, String> {
    let rate = FeeRate::new(input.fee_rate_sat_per_kvb, FALLBACK_MIN_RELAY_SAT_PER_KVB)
        .map_err(|e| serialize_send_error(&SendError::from(e)))?;
    let svc = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    svc.estimate_send(&input, rate)
        .await
        .map_err(|e| serialize_send_error(&e))
}

/// Sends BTC from the Admin Wallet (Phase 6, PRD §4.3.5).
///
/// Validates the fee rate, syncs best-effort so the UTXO view is fresh, then
/// builds/signs/broadcasts through `WalletService::send_to_address` (session
/// `PsbtSigner`, Electrum-first broadcast with node fallback) and re-syncs so
/// the panel converges (balance drops, pending list shows the send).
#[tauri::command]
pub async fn admin_wallet_send(
    input: SendInput,
    wallet_session: tauri::State<'_, WalletSession>,
    node_config: tauri::State<'_, NodeConfigState>,
) -> Result<SendResultDto, String> {
    // Validate the rate before any wallet or network work.
    let rate = FeeRate::new(input.fee_rate_sat_per_kvb, FALLBACK_MIN_RELAY_SAT_PER_KVB)
        .map_err(|e| serialize_send_error(&SendError::from(e)))?;

    let svc = wallet_session
        .current_or_fallback()
        .map_err(serialize_wallet_error)?;
    let cfg = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .clone();
    let btc_rpc: std::sync::Arc<dyn BitcoinRpcClient> = std::sync::Arc::new(
        HttpBitcoinRpcClient::new(cfg.btc_rpc_url(), cfg.btc_rpc_user(), cfg.btc_rpc_pass()),
    );
    // Broadcaster chain: Electrum first, node fallback — same order as governance broadcast.
    let broadcasters: Vec<std::sync::Arc<dyn TxBroadcaster>> = vec![
        std::sync::Arc::new(ElectrumBroadcaster::new(cfg.electrum_url())),
        std::sync::Arc::new(NodeBroadcaster::new(std::sync::Arc::clone(&btc_rpc))),
    ];

    // Best-effort pre-sync: a stale UTXO view is ultimately caught by the network,
    // which rejects spends of confirmed-elsewhere or missing inputs.
    if let Err(e) = svc.sync().await {
        tracing::warn!(error = %e, "pre-send sync failed; proceeding with last-known wallet state");
    }

    let result = svc
        .send_to_address(&input, rate, &broadcasters)
        .await
        .map_err(|e| serialize_send_error(&e))?;

    // Best-effort post-sync so the panel reflects the send immediately.
    if let Err(e) = svc.sync().await {
        tracing::warn!(error = %e, "post-send sync failed; panel will converge on next sync");
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use desktop_app::application::wallet_session::WalletSession;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

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

    #[test]
    fn capability_signer_kind_maps_hw_vendors_to_hardware() {
        assert_eq!(super::capability_signer_kind("ledger"), "hardware");
        assert_eq!(super::capability_signer_kind("trezor"), "hardware");
        assert_eq!(super::capability_signer_kind("mnemonic"), "mnemonic");
        assert_eq!(super::capability_signer_kind("unknown"), "none");
    }

    #[test]
    fn hw_device_type_exposes_specific_device_for_verify_dispatch() {
        assert_eq!(super::hw_device_type("trezor"), Some("trezor".to_string()));
        assert_eq!(super::hw_device_type("ledger"), Some("ledger".to_string()));
        assert_eq!(super::hw_device_type("mnemonic"), None);
        assert_eq!(super::hw_device_type("none"), None);
    }

    #[test]
    fn network_token_maps_each_network() {
        use bdk_wallet::bitcoin::Network;
        assert_eq!(super::network_token(Network::Regtest), "regtest");
        assert_eq!(super::network_token(Network::Testnet), "testnet");
        assert_eq!(super::network_token(Network::Signet), "signet");
        assert_eq!(super::network_token(Network::Bitcoin), "bitcoin");
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

    /// Compile-time check: admin_wallet_next_receive_address takes State<WalletSession>.
    #[test]
    #[allow(clippy::let_underscore_future)]
    fn next_receive_address_command_uses_wallet_session_state() {
        fn _check(s: tauri::State<'_, WalletSession>) {
            let _ = admin_wallet_next_receive_address(s);
        }
        let _ = _check;
    }

    /// Disabled session: the no-session path surfaces the tagged `Disabled` error that
    /// admin_wallet_next_receive_address returns via `current_or_fallback` + `serialize_wallet_error`.
    #[test]
    fn next_receive_address_no_session_surfaces_tagged_disabled_error() {
        let session = WalletSession::empty();
        let err = match session.current_or_fallback() {
            Err(e) => e,
            Ok(_) => panic!("empty session must yield Disabled"),
        };
        let json = serialize_wallet_error(err);
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON object");
        assert_eq!(
            value["type"], "Disabled",
            "no-session next_receive_address must surface the tagged Disabled error"
        );
    }

    // ---- Phase 5: unconfirmed tx list + fee bump (PRD §4.3.3) ----

    /// Both Phase 5 commands must be registered in BOTH handler sets — capability is
    /// enforced per-signer at runtime (`allowed_on(network)`), same as broadcast.
    #[test]
    fn phase5_commands_registered_in_both_handler_sets() {
        let invoke_src = include_str!("invoke.rs");
        for command in ["admin_wallet_list_unconfirmed_txs", "admin_wallet_bump_fee"] {
            let occurrences = invoke_src
                .split_whitespace()
                .filter(|tok| tok.trim_matches(',') == format!("super::admin_wallet::{command}"))
                .count();
            assert_eq!(
                occurrences, 2,
                "{command} must appear in attach_production AND attach_with_dev_signing"
            );
        }
    }

    /// IPC contract: bump errors serialize as the tagged `{ type, message }` shape.
    #[test]
    fn serialize_bump_error_emits_tagged_type_and_message() {
        let json = serialize_bump_error(&BumpFeeError::TxNotReplaceable {
            txid: "ab".to_string(),
        });
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON object");
        assert_eq!(value["type"], "TxNotReplaceable");
        assert!(value["message"]
            .as_str()
            .expect("message string")
            .contains("does not signal RBF"));
    }

    /// An out-of-range rate must serialize as the tagged InvalidFeeRate error
    /// (the command validates before touching the wallet).
    #[test]
    fn invalid_fee_rate_serializes_as_tagged_invalid_fee_rate() {
        let err = FeeRate::new(0, FALLBACK_MIN_RELAY_SAT_PER_KVB)
            .map(|_| ())
            .expect_err("zero rate must be rejected");
        let json = serialize_bump_error(&BumpFeeError::from(err));
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON object");
        assert_eq!(value["type"], "InvalidFeeRate");
    }

    /// BumpFeeInput deserializes from the camelCase IPC payload.
    #[test]
    fn bump_fee_input_deserializes_camel_case() {
        let input: BumpFeeInput =
            serde_json::from_value(serde_json::json!({ "txid": "ab", "feeRateSatPerKvb": 5000 }))
                .expect("deserialize");
        assert_eq!(input.txid, "ab");
        assert_eq!(input.fee_rate_sat_per_kvb, 5_000);
    }

    // ---- Phase 6: Send BTC (PRD §4.3.5) ----

    /// The Phase 6 send command must be registered in BOTH handler sets — capability
    /// is enforced per-signer at runtime (`allowed_on(network)`), same as Phase 5.
    /// (P6.2/P6.3 extend this list with the validate/estimate commands.)
    #[test]
    fn phase6_commands_registered_in_both_handler_sets() {
        let invoke_src = include_str!("invoke.rs");
        for command in [
            "admin_wallet_send",
            "admin_wallet_validate_send_address",
            "admin_wallet_estimate_send",
        ] {
            let occurrences = invoke_src
                .split_whitespace()
                .filter(|tok| tok.trim_matches(',') == format!("super::admin_wallet::{command}"))
                .count();
            assert_eq!(
                occurrences, 2,
                "{command} must appear in attach_production AND attach_with_dev_signing"
            );
        }
    }

    /// IPC contract: send errors serialize as the tagged `{ type, message }` shape.
    #[test]
    fn serialize_send_error_emits_tagged_type_and_message() {
        let json = serialize_send_error(&SendError::WrongNetwork {
            address: "bc1q…".to_string(),
            expected_network: "regtest".to_string(),
        });
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON object");
        assert_eq!(value["type"], "WrongNetwork");
        assert!(value["message"]
            .as_str()
            .expect("message string")
            .contains("regtest"));

        let json = serialize_send_error(&SendError::InvalidAmount);
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON object");
        assert_eq!(value["type"], "InvalidAmount");
    }

    /// An out-of-range rate must serialize as the tagged InvalidFeeRate error
    /// (the command validates before touching the wallet).
    #[test]
    fn send_invalid_fee_rate_serializes_as_tagged_invalid_fee_rate() {
        let err = FeeRate::new(0, FALLBACK_MIN_RELAY_SAT_PER_KVB)
            .map(|_| ())
            .expect_err("zero rate must be rejected");
        let json = serialize_send_error(&SendError::from(err));
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON object");
        assert_eq!(value["type"], "InvalidFeeRate");
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
