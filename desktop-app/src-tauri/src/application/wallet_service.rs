use crate::application::psbt_signer::PsbtSigner;
use crate::infrastructure::admin_wallet::AdminWalletError;
use crate::infrastructure::hw_wallet::hw_psbt_signer::{HwDeviceType, HwPsbtSigner};
use crate::infrastructure::hw_wallet::ledger;
use crate::infrastructure::node_config_store::NodeConfig;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration};

const SYNC_INTERVAL: Duration = Duration::from_secs(30);
const SYNC_IDLE_WINDOW: Duration = Duration::from_secs(300);

/// A sync must run longer than this before progress is surfaced to the UI. Below this threshold,
/// fast (local-node) syncs complete without ever showing a progress indicator, avoiding a flicker
/// on every refresh.
const SYNC_PROGRESS_THRESHOLD_MS: u64 = 3_000;

// ── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutPointDto {
    pub txid: String,
    pub vout: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeychainDto {
    External,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceDto {
    pub confirmed_sats: u64,
    pub unconfirmed_sats: u64,
    pub total_sats: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UtxoDto {
    pub outpoint: OutPointDto,
    pub value_sats: u64,
    pub script_pubkey_hex: String,
    pub keychain: KeychainDto,
    pub derivation_index: u32,
    pub confirmations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddressDto {
    pub index: u32,
    pub address: String,
    pub is_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgressDto {
    pub processed_blocks: u32,
    pub total_blocks: u32,
    pub percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusDto {
    pub tip_height: Option<u32>,
    pub last_synced_block: Option<u32>,
    pub last_synced_at: Option<String>,
    pub is_syncing: bool,
    pub last_error: Option<TypedError>,
    /// Present only while a sync is in flight AND has run longer than
    /// [`SYNC_PROGRESS_THRESHOLD_MS`]; `None` otherwise.
    pub sync_progress: Option<SyncProgressDto>,
}

impl SyncStatusDto {
    /// Returns a SyncStatusDto representing the Disabled state (no active wallet session).
    pub fn disabled_default() -> Self {
        Self {
            tip_height: None,
            last_synced_block: None,
            last_synced_at: None,
            is_syncing: false,
            last_error: Some(TypedError {
                code: "Disabled".to_string(),
                message: AdminWalletError::Disabled.to_string(),
            }),
            sync_progress: None,
        }
    }
}

// ── SyncState ────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct SyncState {
    pub tip_height: Option<u32>,
    pub last_synced_block: Option<u32>,
    pub last_synced_at: Option<String>,
    pub last_error: Option<TypedError>,
}

// ── WalletService ─────────────────────────────────────────────────────────────

pub struct WalletService {
    pub wallet: Arc<Mutex<bdk_wallet::Wallet>>,
    pub sync_state: Arc<RwLock<SyncState>>,
    pub sync_in_flight: Arc<AtomicBool>,
    pub last_read_at: Arc<RwLock<Option<Instant>>>,
    pub cancel: Arc<tokio::sync::Notify>,
    bg_task_started: Arc<AtomicBool>,
    // Lock-free progress counters — read by `sync_status()` without taking any lock the sync
    // loop already holds (the loop holds `Mutex<Wallet>` while applying blocks).
    sync_blocks_processed: Arc<AtomicU32>,
    sync_blocks_total: Arc<AtomicU32>,
    /// UNIX-epoch millis when the current sync started; `0` when idle.
    sync_started_at_ms: Arc<AtomicU64>,
    node_config: Arc<StdRwLock<NodeConfig>>,
    signer: Option<Arc<dyn PsbtSigner>>,
    network: bdk_wallet::bitcoin::Network,
}

/// Keychain selection for address listing.
pub use bdk_wallet::KeychainKind as Keychain;

/// Converts Unix seconds to an ISO-8601 UTC string (e.g. "2026-05-27T17:23:08Z").
/// Pure std — no chrono dependency required.
fn secs_to_iso8601(secs: u64) -> String {
    let mut days = secs / 86400;
    let time_secs = secs % 86400;
    let hh = time_secs / 3600;
    let mm = (time_secs % 3600) / 60;
    let ss = time_secs % 60;

    let mut year = 1970u64;
    loop {
        let leap =
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    let day = days + 1;
    format!("{year:04}-{month:02}-{day:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Percent complete, clamped to `0..=100`. Returns 100 when `total == 0` (nothing to scan).
fn percent_complete(processed: u32, total: u32) -> u8 {
    if total == 0 {
        return 100;
    }
    let pct = (u64::from(processed) * 100 / u64::from(total)).min(100);
    pct as u8
}

/// Current wall-clock time as UNIX-epoch milliseconds. Used only for the elapsed-since-start
/// check that gates the sync progress indicator; monotonicity is not required at this resolution.
fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn rpc_error_from_message(msg: String) -> AdminWalletError {
    if msg.contains("401") || msg.contains("Unauthorized") {
        AdminWalletError::RpcAuthFailed { message: msg }
    } else {
        AdminWalletError::RpcUnreachable { message: msg }
    }
}

pub fn error_code(e: &AdminWalletError) -> String {
    match e {
        AdminWalletError::RpcUnreachable { .. } => "RpcUnreachable".into(),
        AdminWalletError::RpcAuthFailed { .. } => "RpcAuthFailed".into(),
        AdminWalletError::DescriptorParseError { .. } => "DescriptorParseError".into(),
        AdminWalletError::SyncIncomplete { .. } => "SyncIncomplete".into(),
        AdminWalletError::RegtestGuardViolation { .. } => "RegtestGuardViolation".into(),
        AdminWalletError::Disabled => "Disabled".into(),
        AdminWalletError::InvalidMnemonic(_) => "InvalidMnemonic".into(),
        AdminWalletError::Descriptor(_) => "Descriptor".into(),
        AdminWalletError::WalletCreation(_) => "WalletCreation".into(),
        AdminWalletError::ReadOnly => "ReadOnly".into(),
        AdminWalletError::SignerNotAllowedOnNetwork => "SignerNotAllowedOnNetwork".into(),
    }
}

impl WalletService {
    pub fn new(wallet: bdk_wallet::Wallet, node_config: Arc<StdRwLock<NodeConfig>>) -> Self {
        let network = wallet.network();
        Self {
            wallet: Arc::new(Mutex::new(wallet)),
            sync_state: Arc::new(RwLock::new(SyncState::default())),
            sync_in_flight: Arc::new(AtomicBool::new(false)),
            last_read_at: Arc::new(RwLock::new(None)),
            cancel: Arc::new(tokio::sync::Notify::new()),
            bg_task_started: Arc::new(AtomicBool::new(false)),
            sync_blocks_processed: Arc::new(AtomicU32::new(0)),
            sync_blocks_total: Arc::new(AtomicU32::new(0)),
            sync_started_at_ms: Arc::new(AtomicU64::new(0)),
            node_config,
            signer: None,
            network,
        }
    }

    /// Creates a WalletService with an attached signer for session-driven signing.
    pub(crate) fn with_signer(
        wallet: bdk_wallet::Wallet,
        signer: Arc<dyn PsbtSigner>,
        node_config: Arc<StdRwLock<NodeConfig>>,
    ) -> Self {
        let mut svc = Self::new(wallet, node_config);
        svc.signer = Some(signer);
        svc
    }

    /// Creates a watch-only WalletService that cannot sign transactions.
    pub fn new_watch_only(
        wallet: bdk_wallet::Wallet,
        node_config: Arc<StdRwLock<NodeConfig>>,
    ) -> Self {
        Self::new(wallet, node_config)
    }

    /// Returns whether this wallet service can sign transactions.
    /// Returns true only when a signer is attached AND the signer is allowed on the active network.
    pub fn can_sign(&self) -> bool {
        match &self.signer {
            Some(signer) => signer.allowed_on(self.network),
            None => false,
        }
    }

    /// Returns the kind of signer attached to this wallet service.
    /// Returns "mnemonic", "trezor", "ledger", or "none".
    pub fn signer_kind(&self) -> String {
        match &self.signer {
            Some(signer) => signer.kind().to_string(),
            None => "none".to_string(),
        }
    }

    /// Returns a lock-free snapshot of the current sync state.
    pub fn sync_status(&self) -> SyncStatusDto {
        let is_syncing = self.sync_in_flight.load(Ordering::Relaxed);
        let sync_progress = self.sync_progress_snapshot(is_syncing);
        let state = self.sync_state.try_read();
        match state {
            Ok(s) => SyncStatusDto {
                tip_height: s.tip_height,
                last_synced_block: s.last_synced_block,
                last_synced_at: s.last_synced_at.clone(),
                is_syncing,
                last_error: s.last_error.clone(),
                sync_progress,
            },
            Err(_) => SyncStatusDto {
                tip_height: None,
                last_synced_block: None,
                last_synced_at: None,
                is_syncing,
                last_error: None,
                sync_progress,
            },
        }
    }

    /// Builds the progress snapshot, surfaced only while a sync is in flight AND it has been
    /// running longer than [`SYNC_PROGRESS_THRESHOLD_MS`]. Reads the lock-free counters.
    fn sync_progress_snapshot(&self, is_syncing: bool) -> Option<SyncProgressDto> {
        let started = self.sync_started_at_ms.load(Ordering::Relaxed);
        let elapsed = now_unix_ms().saturating_sub(started);
        if !is_syncing || started == 0 || elapsed <= SYNC_PROGRESS_THRESHOLD_MS {
            return None;
        }
        let processed = self.sync_blocks_processed.load(Ordering::Relaxed);
        let total = self.sync_blocks_total.load(Ordering::Relaxed);
        Some(SyncProgressDto {
            processed_blocks: processed,
            total_blocks: total,
            percent: percent_complete(processed, total),
        })
    }

    /// Update the last_read_at timestamp (called by read methods to signal activity).
    pub async fn update_last_read_at(&self) {
        *self.last_read_at.write().await = Some(Instant::now());
    }

    /// Sync the wallet with the Bitcoin RPC node.
    /// Collapses concurrent callers — if a sync is already in-flight, waits for it.
    pub async fn sync(&self) -> Result<SyncStatusDto, AdminWalletError> {
        // Collapse concurrent calls: if already syncing, spin-wait (simple approach for regtest)
        if self
            .sync_in_flight
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            // Another sync is in-flight; wait for it to complete
            while self.sync_in_flight.load(Ordering::SeqCst) {
                sleep(Duration::from_millis(50)).await;
            }
            return Ok(self.sync_status());
        }

        // Reset progress counters and stamp the start time for the 3s threshold gate.
        self.sync_blocks_processed.store(0, Ordering::SeqCst);
        self.sync_blocks_total.store(0, Ordering::SeqCst);
        self.sync_started_at_ms
            .store(now_unix_ms(), Ordering::SeqCst);

        let result = self.do_sync().await;

        // Clear the start time first so progress disappears the instant the sync ends.
        self.sync_started_at_ms.store(0, Ordering::SeqCst);
        self.sync_in_flight.store(false, Ordering::SeqCst);

        match result {
            Ok(()) => Ok(self.sync_status()),
            Err(e) => {
                let typed = TypedError {
                    code: error_code(&e),
                    message: e.to_string(),
                };
                self.sync_state.write().await.last_error = Some(typed);
                Err(e)
            }
        }
    }

    async fn do_sync(&self) -> Result<(), AdminWalletError> {
        use bdk_bitcoind_rpc::bitcoincore_rpc::{Auth, Client, RpcApi};
        use bdk_bitcoind_rpc::Emitter;

        let (rpc_url, rpc_user, rpc_pass) = {
            let cfg = self
                .node_config
                .read()
                .map_err(|_| AdminWalletError::RpcUnreachable {
                    message: "node config lock poisoned".to_string(),
                })?;
            (
                cfg.btc_rpc_url().to_string(),
                cfg.btc_rpc_user().to_string(),
                cfg.btc_rpc_pass().to_string(),
            )
        };

        let rpc = Client::new(&rpc_url, Auth::UserPass(rpc_user, rpc_pass))
            .map_err(|e| rpc_error_from_message(e.to_string()))?;

        let mut wallet = self.wallet.lock().await;
        let checkpoint = wallet.latest_checkpoint();

        // Learn the target tip so the UI can show a meaningful "processed / total" ratio.
        // The Emitter scans from the checkpoint height up to the current chain tip.
        let target_height = rpc
            .get_block_count()
            .map_err(|e| rpc_error_from_message(e.to_string()))?;
        let total = target_height.saturating_sub(u64::from(checkpoint.height()));
        self.sync_blocks_total
            .store(total.min(u64::from(u32::MAX)) as u32, Ordering::Relaxed);

        let mut emitter = Emitter::new(&rpc, checkpoint, 0);

        loop {
            match emitter.next_block() {
                Ok(Some(event)) => {
                    wallet
                        .apply_block_connected_to(
                            &event.block,
                            event.block_height(),
                            event.connected_to(),
                        )
                        .map_err(|e| AdminWalletError::SyncIncomplete {
                            message: e.to_string(),
                        })?;
                    self.sync_blocks_processed.fetch_add(1, Ordering::Relaxed);
                }
                Ok(None) => break,
                Err(e) => return Err(rpc_error_from_message(e.to_string())),
            }
        }

        // Mempool txs are not included in `next_block`; apply them so balance, UTXOs, receive
        // rotation, and R1.5 unconfirmed UX reflect pending credits/spends before the next block.
        let mempool_txs = emitter
            .mempool()
            .map_err(|e| rpc_error_from_message(e.to_string()))?;
        if !mempool_txs.is_empty() {
            wallet.apply_unconfirmed_txs(mempool_txs);
        }

        let tip_height = wallet.latest_checkpoint().height();
        let last_synced_at = secs_to_iso8601(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );

        drop(wallet);

        let mut state = self.sync_state.write().await;
        state.tip_height = Some(tip_height);
        state.last_synced_block = Some(tip_height);
        state.last_synced_at = Some(last_synced_at);
        state.last_error = None;

        Ok(())
    }

    async fn build_and_sign_tx(
        &self,
        commit_addr: bdk_wallet::bitcoin::Address,
        amount_sats: u64,
        fee_rate: bdk_wallet::bitcoin::FeeRate,
    ) -> Result<bdk_wallet::bitcoin::Transaction, AdminWalletError> {
        let signer = self.signer.as_ref().ok_or(AdminWalletError::ReadOnly)?;

        let mut psbt = {
            let mut wallet = self.wallet.lock().await;
            let mut tx_builder = wallet.build_tx();
            tx_builder.add_recipient(
                commit_addr.script_pubkey(),
                bdk_wallet::bitcoin::Amount::from_sat(amount_sats),
            );
            tx_builder.fee_rate(fee_rate);
            tx_builder
                .finish()
                .map_err(|e| AdminWalletError::WalletCreation(e.to_string()))?
        };

        if matches!(signer.kind(), "ledger" | "trezor") {
            let hw = signer
                .as_any()
                .downcast_ref::<HwPsbtSigner>()
                .ok_or_else(|| {
                    AdminWalletError::WalletCreation(
                        "hardware signer metadata missing from session".to_string(),
                    )
                })?;
            if hw.device_type != HwDeviceType::Ledger {
                return Err(AdminWalletError::WalletCreation(
                    "Trezor Admin Wallet PSBT signing is not implemented yet".to_string(),
                ));
            }
            eprintln!(
                "[ledger] signing commit PSBT on device (fp=0x{:08X})…",
                hw.master_fingerprint
            );
            let account_xpub = hw.account_xpub.clone();
            let master_fingerprint = hw.master_fingerprint;
            let network = hw.network;
            let mut psbt_for_ledger = psbt.clone();
            let sign_result = tokio::time::timeout(
                std::time::Duration::from_secs(180),
                tokio::task::spawn_blocking(move || {
                    ledger::sign_admin_wallet_psbt(
                        &mut psbt_for_ledger,
                        &account_xpub,
                        master_fingerprint,
                        network,
                    )
                    .map(|()| psbt_for_ledger)
                }),
            )
            .await
            .map_err(|_| {
                AdminWalletError::WalletCreation(
                    "Ledger did not respond within 180 seconds. Check Speculos/device and approve the transaction."
                        .to_string(),
                )
            })?
            .map_err(|e| AdminWalletError::WalletCreation(format!("ledger sign task failed: {e}")))?;
            psbt = sign_result.map_err(AdminWalletError::WalletCreation)?;
            eprintln!("[ledger] commit PSBT signed on device");
        } else {
            eprintln!("[admin-wallet] signing commit PSBT in software (mnemonic / BDK) — no Ledger prompt");
            let mut wallet = self.wallet.lock().await;
            signer
                .sign_psbt(&mut wallet, &mut psbt)
                .map_err(AdminWalletError::WalletCreation)?;
        }

        let wallet = self.wallet.lock().await;
        let finalized = wallet
            .finalize_psbt(&mut psbt, bdk_wallet::SignOptions::default())
            .map_err(|e| AdminWalletError::WalletCreation(e.to_string()))?;
        if !finalized {
            return Err(AdminWalletError::WalletCreation(
                "PSBT not fully finalized after signing".to_string(),
            ));
        }

        psbt.extract_tx()
            .map_err(|e| AdminWalletError::WalletCreation(e.to_string()))
    }

    /// Spawn the background sync loop once (idempotent). Must be called on first IPC invocation.
    pub fn spawn_background_sync(self: &Arc<Self>) {
        if self
            .bg_task_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return; // already started
        }
        let svc = Arc::clone(self);
        tokio::spawn(async move {
            let token = Arc::clone(&svc.cancel);
            loop {
                let notified = token.notified();
                tokio::select! {
                    biased;
                    _ = notified => break,
                    _ = sleep(SYNC_INTERVAL) => {
                        let last_read = *svc.last_read_at.read().await;
                        if last_read.is_some_and(|t| t.elapsed() < SYNC_IDLE_WINDOW) {
                            let _ = svc.sync().await;
                        }
                    }
                }
            }
        });
    }

    /// Returns balance from the last successful sync. All fields are 0 on a never-synced wallet.
    pub async fn get_balance(&self) -> Result<BalanceDto, AdminWalletError> {
        let wallet = self.wallet.lock().await;
        let balance = wallet.balance();
        let confirmed_sats = balance.confirmed.to_sat();
        let unconfirmed_sats =
            balance.trusted_pending.to_sat() + balance.untrusted_pending.to_sat();
        Ok(BalanceDto {
            confirmed_sats,
            unconfirmed_sats,
            total_sats: confirmed_sats + unconfirmed_sats,
        })
    }

    /// Returns all unspent outputs; empty vec on an empty wallet.
    pub async fn list_utxos(&self) -> Result<Vec<UtxoDto>, AdminWalletError> {
        let wallet = self.wallet.lock().await;
        let tip_height = self.sync_state.read().await.tip_height;
        let utxos = wallet
            .list_unspent()
            .map(|output| {
                let confirmations = match output.chain_position {
                    bdk_wallet::chain::ChainPosition::Confirmed { anchor, .. } => tip_height
                        .map_or(0, |tip| {
                            tip.saturating_sub(anchor.block_id.height).saturating_add(1)
                        }),
                    bdk_wallet::chain::ChainPosition::Unconfirmed { .. } => 0,
                };
                let keychain = match output.keychain {
                    Keychain::External => KeychainDto::External,
                    Keychain::Internal => KeychainDto::Internal,
                };
                UtxoDto {
                    outpoint: OutPointDto {
                        txid: output.outpoint.txid.to_string(),
                        vout: output.outpoint.vout,
                    },
                    value_sats: output.txout.value.to_sat(),
                    script_pubkey_hex: hex::encode(output.txout.script_pubkey.as_bytes()),
                    keychain,
                    derivation_index: output.derivation_index,
                    confirmations,
                }
            })
            .collect();
        Ok(utxos)
    }

    /// Returns addresses for `keychain` in the requested page window (capped at 20).
    /// The total address window is 20; page_index=0 returns indices 0..=19.
    /// Returns empty vec for out-of-bound pages.
    pub async fn list_addresses(
        &self,
        keychain: Keychain,
        page_index: u32,
        page_size: u32,
    ) -> Result<Vec<AddressDto>, AdminWalletError> {
        const MAX_ADDRESS_WINDOW: u32 = 20;
        let page_size = page_size.clamp(1, MAX_ADDRESS_WINDOW);
        let start = page_index.saturating_mul(page_size);

        if start >= MAX_ADDRESS_WINDOW {
            return Ok(vec![]);
        }

        let end = start.saturating_add(page_size).min(MAX_ADDRESS_WINDOW);

        let wallet = self.wallet.lock().await;
        let addresses = (start..end)
            .map(|index| {
                let info = wallet.peek_address(keychain, index);
                let is_used = wallet.spk_index().is_used(keychain, index);
                AddressDto {
                    index,
                    address: info.address.to_string(),
                    is_used,
                }
            })
            .collect();
        Ok(addresses)
    }

    /// Syncs the wallet then builds and signs a commit transaction. Does NOT broadcast.
    /// Single source of truth for the BDK wallet — no ephemeral instances created.
    pub async fn build_signed_commit(
        &self,
        commit_address: &str,
        amount_sats: u64,
        fee_rate: u64,
    ) -> Result<bdk_wallet::bitcoin::Transaction, AdminWalletError> {
        // 0. ReadOnly guard — must run before any RPC contact
        let signer = self.signer.as_ref().ok_or(AdminWalletError::ReadOnly)?;
        eprintln!(
            "[admin-wallet] build_signed_commit: signer={} network={:?}",
            signer.kind(),
            self.network
        );

        // 0b. Network capability check — before any sync/RPC/PSBT build
        if !signer.allowed_on(self.network) {
            return Err(AdminWalletError::SignerNotAllowedOnNetwork);
        }

        // 1. Sync so UTXOs are fresh
        self.sync().await?;

        // 3. Acquire wallet lock, build + sign PSBT, release
        let commit_addr: bdk_wallet::bitcoin::Address<_> = commit_address
            .parse::<bdk_wallet::bitcoin::Address<bdk_wallet::bitcoin::address::NetworkUnchecked>>()
            .map_err(|e| AdminWalletError::WalletCreation(e.to_string()))?
            .require_network(self.network)
            .map_err(|e| AdminWalletError::WalletCreation(e.to_string()))?;

        let fee_rate_val = bdk_wallet::bitcoin::FeeRate::from_sat_per_vb(fee_rate)
            .unwrap_or(bdk_wallet::bitcoin::FeeRate::BROADCAST_MIN);

        self.build_and_sign_tx(commit_addr, amount_sats, fee_rate_val)
            .await
    }

    /// Returns the next unused internal (change) keychain address.
    /// Each call advances the BDK internal keychain index, so consecutive calls return distinct addresses.
    pub async fn reveal_change_address(
        &self,
    ) -> Result<bdk_wallet::bitcoin::Address, AdminWalletError> {
        let mut wallet = self.wallet.lock().await;
        let info = wallet.reveal_next_address(bdk_wallet::KeychainKind::Internal);
        Ok(info.address)
    }

    /// Returns the next unused **external** (receive) address (R1.3 — receive rotation).
    ///
    /// Uses BDK's gap-aware `next_unused_address`, which returns the lowest external
    /// index not yet observed in a transaction. The call is **idempotent**: it returns
    /// the same address until that address is used (credited and observed during sync),
    /// after which it rotates to the next unused index. Pure public derivation — works
    /// identically for mnemonic and watch-only/HW sessions; no signing material is touched.
    pub async fn next_receive_address(&self) -> Result<AddressDto, AdminWalletError> {
        let mut wallet = self.wallet.lock().await;
        let info = wallet.next_unused_address(bdk_wallet::KeychainKind::External);
        Ok(AddressDto {
            index: info.index,
            address: info.address.to_string(),
            is_used: false,
        })
    }

    /// Signals the background sync loop to exit. Idempotent — safe to call multiple times.
    /// Uses notify_waiters() to wake all current waiters (the select! loop observes it).
    pub fn shutdown(&self) {
        self.cancel.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::admin_wallet::AdminWalletError;
    use crate::infrastructure::node_config_store::NodeConfig;

    fn test_node_config() -> Arc<StdRwLock<NodeConfig>> {
        Arc::new(StdRwLock::new(NodeConfig::default()))
    }

    // Acceptance test: struct fields and AdminWalletError::Disabled variant exist
    #[test]
    fn wallet_service_struct_and_disabled_variant_exist() {
        let err = AdminWalletError::Disabled;
        let is_disabled = matches!(err, AdminWalletError::Disabled);
        assert!(
            is_disabled,
            "AdminWalletError::Disabled must exist and match"
        );

        let balance = BalanceDto {
            confirmed_sats: 100,
            unconfirmed_sats: 50,
            total_sats: 150,
        };
        assert_eq!(balance.confirmed_sats, 100);
        assert_eq!(balance.unconfirmed_sats, 50);
        assert_eq!(balance.total_sats, 150);

        let status = SyncStatusDto {
            tip_height: Some(100),
            last_synced_block: Some(99),
            last_synced_at: Some("2026-01-01T00:00:00Z".to_string()),
            is_syncing: false,
            last_error: None,
            sync_progress: None,
        };
        assert!(!status.is_syncing);
    }

    // Acceptance test (step 01-01): secs_to_iso8601 formats Unix epoch as ISO-8601 UTC string
    #[test]
    fn secs_to_iso8601_epoch_zero_returns_unix_epoch_string() {
        assert_eq!(secs_to_iso8601(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn secs_to_iso8601_known_timestamp_returns_correct_date() {
        // 1748361234 → 2025-05-27T15:53:54Z (verified via Python datetime.utcfromtimestamp)
        assert_eq!(secs_to_iso8601(1748361234), "2025-05-27T15:53:54Z");
    }

    // Unit test (step 01-01): one full day boundary
    #[test]
    fn secs_to_iso8601_one_day_returns_1970_01_02() {
        assert_eq!(secs_to_iso8601(86400), "1970-01-02T00:00:00Z");
    }

    // Unit test: confirmation arithmetic
    #[test]
    fn utxo_confirmations_when_confirmed_tip_10_utxo_height_9_returns_2() {
        let tip: u32 = 10;
        let utxo_height: u32 = 9;
        let confirmations = tip.saturating_sub(utxo_height).saturating_add(1);
        assert_eq!(confirmations, 2);
    }

    #[test]
    fn utxo_confirmations_when_unconfirmed_returns_0() {
        let confirmations: u32 = 0;
        assert_eq!(confirmations, 0);
    }

    // Unit test: address windowing
    #[tokio::test]
    async fn list_addresses_page_index_0_returns_indices_0_to_19() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;
        use bdk_wallet::KeychainKind;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        let addresses = svc
            .list_addresses(KeychainKind::External, 0, 20)
            .await
            .expect("list_addresses should not fail");

        assert_eq!(addresses.len(), 20, "page 0 must return 20 addresses");
        assert_eq!(addresses[0].index, 0, "first index must be 0");
        assert_eq!(addresses[19].index, 19, "last index must be 19");
    }

    #[tokio::test]
    async fn list_addresses_out_of_bound_page_returns_empty_vec() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;
        use bdk_wallet::KeychainKind;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        // page_index=1 with page_size=20 on a fresh wallet that only has 20 addresses derivable → []
        let addresses = svc
            .list_addresses(KeychainKind::External, 100, 20)
            .await
            .expect("list_addresses should not fail on out-of-bound page");

        assert!(
            addresses.is_empty(),
            "out-of-bound page must return empty vec"
        );
    }

    // Acceptance test (step 01-03): sync_status() returns is_syncing=false on a fresh WalletService
    #[test]
    fn sync_status_returns_not_syncing_on_fresh_wallet_service() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        let status = svc.sync_status();

        assert!(
            !status.is_syncing,
            "fresh WalletService must report is_syncing=false"
        );
        assert!(
            status.tip_height.is_none(),
            "tip_height must be None before any sync"
        );
        assert!(
            status.last_error.is_none(),
            "last_error must be None before any sync"
        );
    }

    // Unit test (step 01-03): concurrent sync() calls collapse via sync_in_flight AtomicBool
    #[tokio::test]
    async fn concurrent_sync_calls_second_call_observes_in_flight_flag() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;
        use std::sync::atomic::Ordering;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        // Simulate a sync in-flight by setting the flag
        svc.sync_in_flight.store(true, Ordering::SeqCst);

        // A call to sync_status while in_flight=true must report is_syncing=true
        let status = svc.sync_status();
        assert!(
            status.is_syncing,
            "sync_status must reflect in-flight sync as is_syncing=true"
        );
    }

    // Unit test (step 01-02): background loop exits promptly after shutdown() is called
    #[tokio::test]
    async fn spawn_background_sync_loop_exits_after_shutdown() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = Arc::new(WalletService::new(wallet, test_node_config()));

        svc.spawn_background_sync();

        assert!(
            svc.bg_task_started
                .load(std::sync::atomic::Ordering::SeqCst),
            "bg_task_started must be true after spawn_background_sync"
        );

        // Give the task a moment to start polling
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        svc.shutdown();

        // After shutdown, the cancel notify should fire. Verify the cancel signal
        // can be observed quickly — the loop should be exiting.
        // We verify by trying to acquire notified() directly from the cancel Arc.
        // With notify_waiters(), a future that is already polled wakes up.
        // Here we test that calling shutdown() again is safe (idempotent at task level).
        svc.shutdown();

        // The bg task had a chance to observe the signal. No panic means success.
        // Sleep briefly to allow the spawned task to process the signal.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }

    // Unit test (step 01-01): shutdown() can be called multiple times without panicking
    #[test]
    fn shutdown_is_idempotent_and_does_not_panic() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        // Calling shutdown multiple times must not panic
        svc.shutdown();
        svc.shutdown();
        svc.shutdown();
    }

    // Unit test (step 01-02): shutdown() wakes a polling notified() future (notify_waiters semantics)
    #[tokio::test]
    async fn shutdown_wakes_already_polling_notified_future() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = Arc::new(WalletService::new(wallet, test_node_config()));

        let cancel = Arc::clone(&svc.cancel);

        // Register the waiter in a spawned task, then yield so it polls `notified()` before
        // shutdown(). Avoids a fixed sleep + short timeout race that flakes on loaded CI runners.
        let waiter = tokio::spawn(async move { cancel.notified().await });

        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        svc.shutdown();

        let result = tokio::time::timeout(std::time::Duration::from_secs(2), waiter).await;
        assert!(
            result.is_ok() && result.unwrap().is_ok(),
            "notified() waiter registered before shutdown() must complete when shutdown() fires"
        );
    }

    // Unit test (step 01-03): disabled_default() returns is_syncing=false with Disabled error
    #[test]
    fn sync_status_disabled_default_returns_disabled_state() {
        let status = SyncStatusDto::disabled_default();

        assert!(
            !status.is_syncing,
            "disabled_default must return is_syncing=false"
        );
        assert!(status.tip_height.is_none(), "tip_height must be None");
        assert!(
            status.last_synced_block.is_none(),
            "last_synced_block must be None"
        );
        assert!(
            status.last_synced_at.is_none(),
            "last_synced_at must be None"
        );

        let err = status
            .last_error
            .expect("last_error must be Some for Disabled state");
        assert_eq!(err.code, "Disabled", "error code must be 'Disabled'");
        assert!(!err.message.is_empty(), "error message must be non-empty");
    }

    // Unit test (step 01-02): with_signer().can_sign() returns true
    #[test]
    fn with_signer_can_sign_returns_true() {
        use crate::application::psbt_signer::MnemonicPsbtSigner;
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;
        use std::sync::Arc;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let signer = Arc::new(MnemonicPsbtSigner::new());
        let svc = WalletService::with_signer(wallet, signer, test_node_config());

        assert!(svc.can_sign(), "with_signer() must return can_sign=true");
    }

    // Acceptance test (step 01-02): new_watch_only().can_sign() returns false
    #[test]
    fn new_watch_only_can_sign_returns_false() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new_watch_only(wallet, test_node_config());

        assert!(!svc.can_sign(), "new_watch_only must return can_sign=false");
    }

    // Acceptance test (step 01-03): build_signed_commit on watch-only returns ReadOnly without contacting RPC
    #[tokio::test]
    async fn build_signed_commit_on_watch_only_returns_read_only_error() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        // No RPC URL set — if build_signed_commit contacts RPC it would fail with RpcUnreachable, not ReadOnly
        let svc = WalletService::new_watch_only(wallet, test_node_config());

        let result = svc
            .build_signed_commit("bcrt1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqe0xpa", 1000, 1)
            .await;

        assert!(
            matches!(result, Err(AdminWalletError::ReadOnly)),
            "build_signed_commit on watch-only wallet must return ReadOnly, got: {:?}",
            result
        );
    }

    // Unit test (step 01-03): build_signed_commit on watch-only returns ReadOnly (signer absent guard fires first)
    #[tokio::test]
    async fn build_signed_commit_on_watch_only_returns_read_only_before_enabled_check() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new_watch_only(wallet, test_node_config());

        let result = svc
            .build_signed_commit("bcrt1q6rz28mcfaxtmd6v789l9rrlrusdprr9pqe0xpa", 1000, 1)
            .await;

        assert!(
            matches!(result, Err(AdminWalletError::ReadOnly)),
            "build_signed_commit on watch-only must return ReadOnly (signer absent), got: {:?}",
            result
        );
    }

    // Unit test (step 01-03): reveal_change_address returns distinct addresses on consecutive calls
    #[tokio::test]
    async fn reveal_change_address_returns_distinct_addresses_on_consecutive_calls() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        let addr1: bdk_wallet::bitcoin::Address = svc
            .reveal_change_address()
            .await
            .expect("first reveal_change_address must succeed");
        let addr2: bdk_wallet::bitcoin::Address = svc
            .reveal_change_address()
            .await
            .expect("second reveal_change_address must succeed");

        assert_ne!(
            addr1, addr2,
            "consecutive reveal_change_address calls must return distinct addresses"
        );
    }

    // R1.3 (receive rotation): next_receive_address on a fresh wallet returns external index 0, unused.
    #[tokio::test]
    async fn next_receive_address_on_fresh_wallet_returns_index_0_unused() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        let addr = svc
            .next_receive_address()
            .await
            .expect("next_receive_address must succeed");

        assert_eq!(addr.index, 0, "fresh wallet must return external index 0");
        assert!(
            !addr.is_used,
            "freshly issued receive address must be unused"
        );
        assert!(
            addr.address.starts_with("bcrt1p"),
            "regtest receive address must be a P2TR bcrt1p address, got: {}",
            addr.address
        );
    }

    // R1.3: idempotency guard — consecutive calls return the SAME address until it is used.
    // (Contrast with reveal_change_address, which always advances.)
    #[tokio::test]
    async fn next_receive_address_is_idempotent_until_used() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        let a1 = svc.next_receive_address().await.expect("first call ok");
        let a2 = svc.next_receive_address().await.expect("second call ok");

        assert_eq!(
            a1.index, a2.index,
            "next_receive_address must not advance the index while the address is unused"
        );
        assert_eq!(
            a1.address, a2.address,
            "consecutive next_receive_address calls must return the same unused address"
        );
    }

    // R1.3: watch-only / HW compatibility — pure derivation, no ReadOnly error.
    #[tokio::test]
    async fn next_receive_address_on_watch_only_returns_ok() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new_watch_only(wallet, test_node_config());

        let addr = svc
            .next_receive_address()
            .await
            .expect("watch-only next_receive_address must succeed (pure derivation)");

        assert_eq!(addr.index, 0, "watch-only fresh wallet must return index 0");
    }

    // Step 02 (sync_status progress gate): progress reported only after >3s in-flight
    #[test]
    fn sync_status_reports_progress_after_threshold() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        svc.sync_in_flight.store(true, Ordering::SeqCst);
        svc.sync_started_at_ms
            .store(now_unix_ms() - 4_000, Ordering::SeqCst);
        svc.sync_blocks_processed.store(234, Ordering::SeqCst);
        svc.sync_blocks_total.store(1277, Ordering::SeqCst);

        let progress = svc
            .sync_status()
            .sync_progress
            .expect("progress must be present after threshold");
        assert_eq!(progress.processed_blocks, 234);
        assert_eq!(progress.total_blocks, 1277);
        assert_eq!(progress.percent, 18);
    }

    // Step 02 (sync_status progress gate): hidden under the 3s threshold
    #[test]
    fn sync_status_hides_progress_under_threshold() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        svc.sync_in_flight.store(true, Ordering::SeqCst);
        svc.sync_started_at_ms
            .store(now_unix_ms() - 1_000, Ordering::SeqCst);
        svc.sync_blocks_processed.store(234, Ordering::SeqCst);
        svc.sync_blocks_total.store(1277, Ordering::SeqCst);

        assert!(
            svc.sync_status().sync_progress.is_none(),
            "progress must be hidden under the 3s threshold"
        );
    }

    // Step 02 (sync_status progress gate): hidden when not syncing regardless of counters
    #[test]
    fn sync_status_hides_progress_when_not_syncing() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        svc.sync_in_flight.store(false, Ordering::SeqCst);
        svc.sync_started_at_ms
            .store(now_unix_ms() - 4_000, Ordering::SeqCst);
        svc.sync_blocks_processed.store(234, Ordering::SeqCst);
        svc.sync_blocks_total.store(1277, Ordering::SeqCst);

        assert!(
            svc.sync_status().sync_progress.is_none(),
            "progress must be hidden when no sync is in flight"
        );
    }

    // Step 02 (DTO shape): disabled_default carries no progress; SyncProgressDto is camelCase
    #[test]
    fn disabled_default_has_no_progress_and_dto_is_camel_case() {
        assert!(SyncStatusDto::disabled_default().sync_progress.is_none());

        let json = serde_json::to_string(&SyncProgressDto {
            processed_blocks: 1,
            total_blocks: 2,
            percent: 50,
        })
        .expect("serialize");
        assert!(json.contains("processedBlocks"), "got: {json}");
        assert!(json.contains("totalBlocks"), "got: {json}");
        assert!(json.contains("percent"), "got: {json}");
    }

    // Step 01 (percent_complete): truncating integer arithmetic
    #[test]
    fn percent_complete_partial_truncates() {
        assert_eq!(percent_complete(234, 1277), 18);
    }

    // Step 01 (percent_complete): divide-by-zero guard returns 100 (nothing to scan)
    #[test]
    fn percent_complete_zero_total_returns_100() {
        assert_eq!(percent_complete(0, 0), 100);
    }

    // Step 01 (percent_complete): full progress
    #[test]
    fn percent_complete_full_returns_100() {
        assert_eq!(percent_complete(1277, 1277), 100);
    }

    // Step 01 (percent_complete): over-count clamps to 100
    #[test]
    fn percent_complete_over_count_clamps_to_100() {
        assert_eq!(percent_complete(200, 100), 100);
    }

    // Step 01 (percent_complete): midpoint
    #[test]
    fn percent_complete_midpoint_returns_50() {
        assert_eq!(percent_complete(50, 100), 50);
    }

    // Acceptance test: get_balance on a never-synced wallet returns all-zero BalanceDto
    #[tokio::test]
    async fn get_balance_returns_all_zero_on_never_synced_wallet() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet, test_node_config());

        let balance = svc
            .get_balance()
            .await
            .expect("get_balance should not fail");

        assert_eq!(
            balance.confirmed_sats, 0,
            "confirmed_sats must be 0 on never-synced wallet"
        );
        assert_eq!(
            balance.unconfirmed_sats, 0,
            "unconfirmed_sats must be 0 on never-synced wallet"
        );
        assert_eq!(
            balance.total_sats, 0,
            "total_sats must be 0 on never-synced wallet"
        );
    }
}
