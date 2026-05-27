use crate::infrastructure::admin_wallet::AdminWalletError;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

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
}

impl WalletService {
    pub fn new(wallet: bdk_wallet::Wallet) -> Self {
        Self {
            wallet: Arc::new(Mutex::new(wallet)),
            sync_state: Arc::new(RwLock::new(SyncState::default())),
            sync_in_flight: Arc::new(AtomicBool::new(false)),
            last_read_at: Arc::new(RwLock::new(None)),
        }
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
}
