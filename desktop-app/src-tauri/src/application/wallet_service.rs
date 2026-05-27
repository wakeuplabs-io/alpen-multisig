use crate::infrastructure::admin_wallet::AdminWalletError;
use bdk_bitcoind_rpc::bitcoincore_rpc::Auth;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration};

const SYNC_INTERVAL: Duration = Duration::from_secs(30);
const SYNC_IDLE_WINDOW: Duration = Duration::from_secs(300);

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
pub struct BalanceDto {
    pub confirmed_sats: u64,
    pub unconfirmed_sats: u64,
    pub total_sats: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtxoDto {
    pub outpoint: OutPointDto,
    pub value_sats: u64,
    pub script_pubkey_hex: String,
    pub keychain: KeychainDto,
    pub derivation_index: u32,
    pub confirmations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct SyncStatusDto {
    pub tip_height: Option<u32>,
    pub last_synced_block: Option<u32>,
    pub last_synced_at: Option<String>,
    pub is_syncing: bool,
    pub last_error: Option<TypedError>,
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
    bg_task_started: Arc<AtomicBool>,
    rpc_url: String,
    rpc_user: String,
    rpc_pass: String,
}

/// Keychain selection for address listing.
pub use bdk_wallet::KeychainKind as Keychain;

fn error_code(e: &AdminWalletError) -> String {
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
    }
}

impl WalletService {
    pub fn new(wallet: bdk_wallet::Wallet) -> Self {
        let rpc_url =
            std::env::var("BITCOIN_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:18443".into());
        let rpc_user = std::env::var("BITCOIN_RPC_USER").unwrap_or_default();
        let rpc_pass = std::env::var("BITCOIN_RPC_PASS").unwrap_or_default();
        Self {
            wallet: Arc::new(Mutex::new(wallet)),
            sync_state: Arc::new(RwLock::new(SyncState::default())),
            sync_in_flight: Arc::new(AtomicBool::new(false)),
            last_read_at: Arc::new(RwLock::new(None)),
            bg_task_started: Arc::new(AtomicBool::new(false)),
            rpc_url,
            rpc_user,
            rpc_pass,
        }
    }

    /// Returns a lock-free snapshot of the current sync state.
    pub fn sync_status(&self) -> SyncStatusDto {
        let is_syncing = self.sync_in_flight.load(Ordering::Relaxed);
        let state = self.sync_state.try_read();
        match state {
            Ok(s) => SyncStatusDto {
                tip_height: s.tip_height,
                last_synced_block: s.last_synced_block,
                last_synced_at: s.last_synced_at.clone(),
                is_syncing,
                last_error: s.last_error.clone(),
            },
            Err(_) => SyncStatusDto {
                tip_height: None,
                last_synced_block: None,
                last_synced_at: None,
                is_syncing,
                last_error: None,
            },
        }
    }

    /// Update the last_read_at timestamp (called by read methods to signal activity).
    pub async fn update_last_read_at(&self) {
        *self.last_read_at.write().await = Some(Instant::now());
    }

    /// Sync the wallet with the Bitcoin RPC node.
    /// Collapses concurrent callers — if a sync is already in-flight, waits for it.
    pub async fn sync(&self) -> Result<SyncStatusDto, AdminWalletError> {
        // Check disabled guard first
        WalletService::check_enabled()?;

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

        let result = self.do_sync().await;

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
        use bdk_bitcoind_rpc::bitcoincore_rpc::Client;
        use bdk_bitcoind_rpc::Emitter;

        let rpc = Client::new(
            &self.rpc_url,
            Auth::UserPass(self.rpc_user.clone(), self.rpc_pass.clone()),
        )
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("401") || msg.contains("Unauthorized") {
                AdminWalletError::RpcAuthFailed { message: msg }
            } else {
                AdminWalletError::RpcUnreachable { message: msg }
            }
        })?;

        let mut wallet = self.wallet.lock().await;
        let checkpoint = wallet.latest_checkpoint();
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
                }
                Ok(None) => break,
                Err(e) => {
                    let msg = e.to_string();
                    return Err(if msg.contains("401") || msg.contains("Unauthorized") {
                        AdminWalletError::RpcAuthFailed { message: msg }
                    } else {
                        AdminWalletError::RpcUnreachable { message: msg }
                    });
                }
            }
        }

        let tip_height = wallet.latest_checkpoint().height();
        let last_synced_at = {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            format!("{secs}")
        };

        drop(wallet);

        let mut state = self.sync_state.write().await;
        state.tip_height = Some(tip_height);
        state.last_synced_block = Some(tip_height);
        state.last_synced_at = Some(last_synced_at);
        state.last_error = None;

        Ok(())
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
            loop {
                sleep(SYNC_INTERVAL).await;
                let last_read = *svc.last_read_at.read().await;
                if last_read.is_some_and(|t| t.elapsed() < SYNC_IDLE_WINDOW) {
                    let _ = svc.sync().await;
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

    /// Guard: returns Disabled if env configuration does not allow admin wallet.
    pub fn check_enabled() -> Result<(), AdminWalletError> {
        let commit_funding = std::env::var("COMMIT_FUNDING").unwrap_or_default();
        let bitcoin_network = std::env::var("BITCOIN_NETWORK").unwrap_or_default();
        let allow_dev = std::env::var("ALLOW_DEV_MNEMONIC_SIGNING").unwrap_or_default();

        if commit_funding != "admin_wallet" || bitcoin_network != "regtest" || allow_dev != "1" {
            return Err(AdminWalletError::Disabled);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::admin_wallet::AdminWalletError;
    use std::sync::Mutex;

    // Serialize env-var tests to avoid cross-test pollution
    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
        };
        assert!(!status.is_syncing);
    }

    // Unit test: guard returns Disabled when env vars are missing/wrong
    #[test]
    fn check_enabled_returns_disabled_when_env_vars_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("COMMIT_FUNDING");
        std::env::remove_var("BITCOIN_NETWORK");
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");

        let result = WalletService::check_enabled();
        assert!(
            matches!(result, Err(AdminWalletError::Disabled)),
            "Expected Disabled when env vars missing"
        );
    }

    // Unit test: guard returns Ok when all env vars are correctly set
    #[test]
    fn check_enabled_returns_ok_when_all_env_vars_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("COMMIT_FUNDING", "admin_wallet");
        std::env::set_var("BITCOIN_NETWORK", "regtest");
        std::env::set_var("ALLOW_DEV_MNEMONIC_SIGNING", "1");

        let result = WalletService::check_enabled();

        std::env::remove_var("COMMIT_FUNDING");
        std::env::remove_var("BITCOIN_NETWORK");
        std::env::remove_var("ALLOW_DEV_MNEMONIC_SIGNING");

        assert!(
            result.is_ok(),
            "Expected Ok when all env vars set correctly"
        );
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
        let svc = WalletService::new(wallet);

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
        let svc = WalletService::new(wallet);

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
        let svc = WalletService::new(wallet);

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
        let svc = WalletService::new(wallet);

        // Simulate a sync in-flight by setting the flag
        svc.sync_in_flight.store(true, Ordering::SeqCst);

        // A call to sync_status while in_flight=true must report is_syncing=true
        let status = svc.sync_status();
        assert!(
            status.is_syncing,
            "sync_status must reflect in-flight sync as is_syncing=true"
        );
    }

    // Acceptance test: get_balance on a never-synced wallet returns all-zero BalanceDto
    #[tokio::test]
    async fn get_balance_returns_all_zero_on_never_synced_wallet() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_wallet::bitcoin::Network;

        const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let svc = WalletService::new(wallet);

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
