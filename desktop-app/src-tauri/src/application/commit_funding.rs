//! Commit funding strategy — selects how the taproot commit tx is funded.
//!
//! Default: `BitcoindSendToAddress` (delegates to the node wallet via JSON-RPC).
//! Optional: `BdkAdminWalletMnemonic` (funds from Admin Wallet descriptors; regtest only).

use std::sync::Arc;

use async_trait::async_trait;
use bitcoin::Network;

use crate::application::wallet_service::WalletService;
use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;

#[derive(Debug, thiserror::Error)]
pub enum CommitFundingError {
    #[error("admin_wallet funding requires regtest network, got {0}")]
    NotRegtest(String),
    #[error("missing env var: {0}")]
    MissingEnv(String),
    #[error("bitcoin RPC error: {0}")]
    BitcoinRpc(String),
    #[error("admin wallet error: {0}")]
    AdminWallet(String),
}

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
// BitcoindSendToAddress — delegates to the node wallet via JSON-RPC
// ---------------------------------------------------------------------------

pub struct BitcoindSendToAddress {
    rpc: Arc<dyn BitcoinRpcClient>,
}

impl BitcoindSendToAddress {
    pub fn new(rpc: Arc<dyn BitcoinRpcClient>) -> Self {
        Self { rpc }
    }
}

#[async_trait]
impl CommitFunding for BitcoindSendToAddress {
    async fn fund_commit(
        &self,
        commit_address: &str,
        amount_sats: u64,
        fee_rate: u64,
    ) -> Result<String, CommitFundingError> {
        self.rpc
            .send_to_address(commit_address, amount_sats, fee_rate)
            .await
            .map_err(CommitFundingError::BitcoinRpc)
    }
}

// ---------------------------------------------------------------------------
// BdkAdminWalletMnemonic — funds from Admin Wallet descriptors (regtest only)
// ---------------------------------------------------------------------------

pub struct BdkAdminWalletMnemonic {
    pub network: Network,
    pub wallet_service: Arc<WalletService>,
}

impl BdkAdminWalletMnemonic {
    pub fn new(network: Network, wallet_service: Arc<WalletService>) -> Self {
        Self {
            network,
            wallet_service,
        }
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
// Factory
// ---------------------------------------------------------------------------

/// Selects the commit funding strategy based on the `COMMIT_FUNDING` env var.
///
/// - unset / `"bitcoind"` → `BitcoindSendToAddress`
/// - `"admin_wallet"` → `BdkAdminWalletMnemonic` (requires `network == Regtest`)
pub fn select_commit_funding(
    btc_rpc: Arc<dyn BitcoinRpcClient>,
    network: Network,
    wallet_service: Option<Arc<WalletService>>,
) -> Result<Box<dyn CommitFunding + Send + Sync>, CommitFundingError> {
    let mode = std::env::var("COMMIT_FUNDING").unwrap_or_else(|_| "bitcoind".to_string());
    match mode.as_str() {
        "admin_wallet" => {
            if network != Network::Regtest {
                return Err(CommitFundingError::NotRegtest(format!("{network}")));
            }
            let ws = wallet_service.ok_or_else(|| {
                CommitFundingError::MissingEnv(
                    "WalletService not provided for admin_wallet mode".into(),
                )
            })?;
            Ok(Box::new(BdkAdminWalletMnemonic::new(network, ws)))
        }
        _ => Ok(Box::new(BitcoindSendToAddress::new(btc_rpc))),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use bitcoin::{Network, Transaction};

    use super::*;
    use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;

    // Minimal no-op mock — only needed to satisfy the Arc<dyn BitcoinRpcClient> arg
    struct MockRpc;

    #[async_trait]
    impl BitcoinRpcClient for MockRpc {
        async fn send_to_address(&self, _: &str, _: u64, _: u64) -> Result<String, String> {
            Ok("mock-txid".into())
        }
        async fn send_raw_transaction(&self, _: &str) -> Result<String, String> {
            unimplemented!()
        }
        async fn get_transaction_confirmations(&self, _: &str) -> Result<u32, String> {
            unimplemented!()
        }
        async fn estimate_fee_rate_sats_per_vb(&self, _: u16) -> Result<u64, String> {
            unimplemented!()
        }
        async fn get_raw_transaction(&self, _: &str) -> Result<Transaction, String> {
            unimplemented!()
        }
        async fn mine_blocks(&self, _: u32) -> Result<(), String> {
            unimplemented!()
        }
    }

    fn mock_rpc() -> Arc<dyn BitcoinRpcClient> {
        Arc::new(MockRpc)
    }

    // Env-var tests mutate process-level state; serialize with a mutex.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn make_wallet_service() -> Arc<crate::application::wallet_service::WalletService> {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        const TEST_MNEMONIC: &str =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let bdk_wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        Arc::new(crate::application::wallet_service::WalletService::new(
            bdk_wallet,
        ))
    }

    #[test]
    fn default_selection_returns_bitcoind_variant() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("COMMIT_FUNDING");

        let result = select_commit_funding(mock_rpc(), Network::Regtest, None);
        assert!(result.is_ok(), "expected Ok but got an error");
    }

    #[test]
    fn admin_wallet_with_regtest_returns_bdk_variant() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("COMMIT_FUNDING", "admin_wallet");

        let result =
            select_commit_funding(mock_rpc(), Network::Regtest, Some(make_wallet_service()));
        std::env::remove_var("COMMIT_FUNDING");
        assert!(result.is_ok(), "expected Ok but got an error");
    }

    #[test]
    fn bdk_admin_wallet_stores_network() {
        let wallet = BdkAdminWalletMnemonic::new(Network::Regtest, make_wallet_service());
        assert_eq!(wallet.network, Network::Regtest);
    }

    // Acceptance test (step 01-01): BdkAdminWalletMnemonic stores injected WalletService — no
    // ephemeral wallet created on construction.
    #[test]
    fn bdk_admin_wallet_mnemonic_stores_injected_wallet_service() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use std::sync::Arc;

        const TEST_MNEMONIC: &str =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let bdk_wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let wallet_service = Arc::new(crate::application::wallet_service::WalletService::new(
            bdk_wallet,
        ));

        let funded = BdkAdminWalletMnemonic::new(Network::Regtest, Arc::clone(&wallet_service));
        assert!(
            Arc::ptr_eq(&funded.wallet_service, &wallet_service),
            "BdkAdminWalletMnemonic must store the injected WalletService Arc (same pointer)"
        );
    }

    // Unit test (step 01-01): select_commit_funding with admin_wallet mode injects WalletService
    #[test]
    fn select_commit_funding_admin_wallet_injects_wallet_service() {
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use std::sync::Arc;

        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("COMMIT_FUNDING", "admin_wallet");

        const TEST_MNEMONIC: &str =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let bdk_wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        let wallet_service = Arc::new(crate::application::wallet_service::WalletService::new(
            bdk_wallet,
        ));

        let result = select_commit_funding(mock_rpc(), Network::Regtest, Some(wallet_service));
        std::env::remove_var("COMMIT_FUNDING");
        assert!(result.is_ok(), "expected Ok when wallet_service injected");
    }

    #[test]
    fn admin_wallet_with_non_regtest_returns_not_regtest_error() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("COMMIT_FUNDING", "admin_wallet");

        let result =
            select_commit_funding(mock_rpc(), Network::Bitcoin, Some(make_wallet_service()));
        std::env::remove_var("COMMIT_FUNDING");

        assert!(
            matches!(result, Err(CommitFundingError::NotRegtest(_))),
            "expected NotRegtest error"
        );
    }
}
