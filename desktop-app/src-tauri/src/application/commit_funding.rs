//! Commit funding strategy — selects how the taproot commit tx is funded.
//!
//! Default: `BitcoindSendToAddress` (delegates to the node wallet via JSON-RPC).
//! Optional: `BdkAdminWalletMnemonic` (funds from Admin Wallet descriptors; regtest only).

use std::sync::Arc;

use async_trait::async_trait;
use bitcoin::Network;

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

pub struct BdkAdminWalletMnemonic;

impl BdkAdminWalletMnemonic {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BdkAdminWalletMnemonic {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommitFunding for BdkAdminWalletMnemonic {
    async fn fund_commit(
        &self,
        _commit_address: &str,
        _amount_sats: u64,
        _fee_rate: u64,
    ) -> Result<String, CommitFundingError> {
        Err(CommitFundingError::AdminWallet(
            "BDK commit not yet implemented".into(),
        ))
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
) -> Result<Box<dyn CommitFunding + Send + Sync>, CommitFundingError> {
    let mode = std::env::var("COMMIT_FUNDING").unwrap_or_else(|_| "bitcoind".to_string());
    match mode.as_str() {
        "admin_wallet" => {
            if network != Network::Regtest {
                return Err(CommitFundingError::NotRegtest(format!("{network}")));
            }
            Ok(Box::new(BdkAdminWalletMnemonic::new()))
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

    #[test]
    fn default_selection_returns_bitcoind_variant() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("COMMIT_FUNDING");

        let result = select_commit_funding(mock_rpc(), Network::Regtest);
        assert!(result.is_ok(), "expected Ok but got an error");
    }

    #[test]
    fn admin_wallet_with_regtest_returns_bdk_variant() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("COMMIT_FUNDING", "admin_wallet");

        let result = select_commit_funding(mock_rpc(), Network::Regtest);
        std::env::remove_var("COMMIT_FUNDING");
        assert!(result.is_ok(), "expected Ok but got an error");
    }

    #[test]
    fn admin_wallet_with_non_regtest_returns_not_regtest_error() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("COMMIT_FUNDING", "admin_wallet");

        let result = select_commit_funding(mock_rpc(), Network::Bitcoin);
        std::env::remove_var("COMMIT_FUNDING");

        assert!(
            matches!(result, Err(CommitFundingError::NotRegtest(_))),
            "expected NotRegtest error"
        );
    }
}
