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

pub struct BdkAdminWalletMnemonic {
    pub network: Network,
}

impl BdkAdminWalletMnemonic {
    pub fn new(network: Network) -> Self {
        Self { network }
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
        use crate::infrastructure::admin_wallet::load_admin_wallet;
        use bdk_bitcoind_rpc::Emitter;

        let mnemonic = std::env::var("ADMIN_WALLET_REGTEST_MNEMONIC")
            .map_err(|_| CommitFundingError::MissingEnv("ADMIN_WALLET_REGTEST_MNEMONIC".into()))?;

        let mut wallet = load_admin_wallet(&mnemonic, self.network)
            .map_err(|e| CommitFundingError::AdminWallet(e.to_string()))?;

        let rpc_url =
            std::env::var("BITCOIN_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:18443".into());
        let rpc_user = std::env::var("BITCOIN_RPC_USER").unwrap_or_default();
        let rpc_pass = std::env::var("BITCOIN_RPC_PASS").unwrap_or_default();

        let rpc = bdk_bitcoind_rpc::bitcoincore_rpc::Client::new(
            &rpc_url,
            bdk_bitcoind_rpc::bitcoincore_rpc::Auth::UserPass(rpc_user, rpc_pass),
        )
        .map_err(|e| CommitFundingError::BitcoinRpc(e.to_string()))?;

        let mut emitter = Emitter::new(&rpc, wallet.latest_checkpoint(), 0);
        while let Some(event) = emitter
            .next_block()
            .map_err(|e| CommitFundingError::BitcoinRpc(e.to_string()))?
        {
            wallet
                .apply_block_connected_to(&event.block, event.block_height(), event.connected_to())
                .map_err(|e| CommitFundingError::AdminWallet(e.to_string()))?;
        }

        let commit_addr: bdk_wallet::bitcoin::Address<_> = commit_address
            .parse::<bdk_wallet::bitcoin::Address<bdk_wallet::bitcoin::address::NetworkUnchecked>>()
            .map_err(|e| CommitFundingError::AdminWallet(e.to_string()))?
            .require_network(self.network)
            .map_err(|e| CommitFundingError::AdminWallet(e.to_string()))?;

        let fee_rate_val = bdk_wallet::bitcoin::FeeRate::from_sat_per_vb(fee_rate)
            .unwrap_or(bdk_wallet::bitcoin::FeeRate::BROADCAST_MIN);

        let mut tx_builder = wallet.build_tx();
        tx_builder.add_recipient(
            commit_addr.script_pubkey(),
            bdk_wallet::bitcoin::Amount::from_sat(amount_sats),
        );
        tx_builder.fee_rate(fee_rate_val);
        let mut psbt = tx_builder
            .finish()
            .map_err(|e| CommitFundingError::AdminWallet(e.to_string()))?;

        wallet
            .sign(&mut psbt, bdk_wallet::SignOptions::default())
            .map_err(|e| CommitFundingError::AdminWallet(e.to_string()))?;

        let tx = psbt
            .extract_tx()
            .map_err(|e| CommitFundingError::AdminWallet(e.to_string()))?;

        use bdk_bitcoind_rpc::bitcoincore_rpc::RpcApi;
        let txid = rpc
            .send_raw_transaction(&tx)
            .map_err(|e| CommitFundingError::BitcoinRpc(e.to_string()))?;

        Ok(txid.to_string())
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
            Ok(Box::new(BdkAdminWalletMnemonic::new(network)))
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
    fn bdk_admin_wallet_stores_network() {
        let wallet = BdkAdminWalletMnemonic::new(Network::Regtest);
        assert_eq!(wallet.network, Network::Regtest);
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
