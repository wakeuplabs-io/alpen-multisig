//! Commit funding via the Admin Wallet (BDK).
//!
//! `BdkAdminWalletMnemonic` is the sole implementor of `CommitFunding`.
//! The legacy `BitcoindSendToAddress` path and the `COMMIT_FUNDING` env-var toggle
//! were removed in Phase 3.6 — the Admin Wallet is the only commit funder.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::wallet_service::WalletService;

#[derive(Debug, thiserror::Error)]
pub enum CommitFundingError {
    #[error("admin wallet error: {0}")]
    AdminWallet(String),
}

/// Abstraction seam for commit funding. Phase 7 HW signing will plug in at the `WalletService`
/// level; this trait's single implementor remains `BdkAdminWalletMnemonic`.
#[async_trait]
pub trait CommitFunding: Send + Sync {
    async fn fund_commit(
        &self,
        commit_address: &str,
        amount_sats: u64,
        fee_rate: u64,
    ) -> Result<String, CommitFundingError>;
}

// ---------------------------------------------------------------------------
// BdkAdminWalletMnemonic — sole commit funder via BDK Admin Wallet
// ---------------------------------------------------------------------------

/// Funds the commit transaction from the Admin Wallet descriptors.
/// Requires `BITCOIN_NETWORK=regtest` and `ALLOW_DEV_MNEMONIC_SIGNING=1`
/// (enforced by `WalletService::check_enabled()` inside `fund_commit`).
pub struct BdkAdminWalletMnemonic {
    pub wallet_service: Arc<WalletService>,
}

impl BdkAdminWalletMnemonic {
    pub fn new(wallet_service: Arc<WalletService>) -> Self {
        Self { wallet_service }
    }
}

#[async_trait]
impl CommitFunding for BdkAdminWalletMnemonic {
    async fn fund_commit(
        &self,
        commit_address: &str,
        amount_sats: u64,
        fee_rate: u64,
    ) -> Result<String, CommitFundingError> {
        self.wallet_service
            .fund_commit(commit_address, amount_sats, fee_rate)
            .await
            .map_err(|e| CommitFundingError::AdminWallet(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::admin_wallet::load_admin_wallet;
    use bdk_wallet::bitcoin::Network;

    fn make_wallet_service() -> Arc<WalletService> {
        const TEST_MNEMONIC: &str =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let bdk_wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        Arc::new(WalletService::new(bdk_wallet))
    }

    // BdkAdminWalletMnemonic stores the injected WalletService — no ephemeral wallet on construction.
    #[test]
    fn bdk_admin_wallet_mnemonic_stores_injected_wallet_service() {
        let wallet_service = make_wallet_service();
        let funder = BdkAdminWalletMnemonic::new(Arc::clone(&wallet_service));
        assert!(
            Arc::ptr_eq(&funder.wallet_service, &wallet_service),
            "BdkAdminWalletMnemonic must store the injected WalletService Arc (same pointer)"
        );
    }
}
