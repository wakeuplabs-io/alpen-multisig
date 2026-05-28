use crate::application::wallet_service::WalletService;
use crate::infrastructure::admin_wallet::{load_admin_wallet, AdminWalletError};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct WalletSession {
    pub(crate) inner: Arc<RwLock<Option<Arc<WalletService>>>>,
}

impl WalletSession {
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
        }
    }

    pub fn current(&self) -> Option<Arc<WalletService>> {
        self.inner.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn clear(&self) {
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(svc) = guard.take() {
            svc.shutdown();
        }
    }

    pub async fn init_from_mnemonic(
        &self,
        mnemonic: &str,
        _passphrase: Option<&str>,
        network: Option<&str>,
    ) -> Result<(), AdminWalletError> {
        let net = parse_network(network);
        let wallet = load_admin_wallet(mnemonic, net)?;
        let svc = Arc::new(WalletService::new(wallet));
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(old) = guard.take() {
            old.shutdown();
        }
        *guard = Some(svc);
        Ok(())
    }
}

fn parse_network(network: Option<&str>) -> bdk_wallet::bitcoin::Network {
    match network.unwrap_or("regtest") {
        "testnet" => bdk_wallet::bitcoin::Network::Testnet,
        "bitcoin" | "mainnet" => bdk_wallet::bitcoin::Network::Bitcoin,
        _ => bdk_wallet::bitcoin::Network::Regtest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::admin_wallet::load_admin_wallet;
    use bdk_wallet::bitcoin::Network;
    use std::sync::Arc;

    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    fn make_service() -> Arc<WalletService> {
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).expect("wallet ok");
        Arc::new(WalletService::new(wallet))
    }

    #[test]
    fn empty_produces_none_current() {
        let session = WalletSession::empty();
        assert!(
            session.current().is_none(),
            "empty() must produce a session whose current() is None"
        );
    }

    #[tokio::test]
    async fn clear_removes_service_and_calls_shutdown() {
        let session = WalletSession::empty();
        let svc = make_service();
        let cancel = Arc::clone(&svc.cancel);

        // Store the service in the slot
        {
            let mut guard = session.inner.write().unwrap_or_else(|e| e.into_inner());
            *guard = Some(Arc::clone(&svc));
        }

        // Verify it is present before clear
        assert!(
            session.current().is_some(),
            "service must be present before clear()"
        );

        // Register notified() BEFORE calling clear() — notify_waiters() only wakes
        // futures that are already registered at the moment of the call.
        let notified = cancel.notified();

        // Spawn clear on a separate task so notified() can be polled concurrently.
        let session_clone = session.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            session_clone.clear();
        });

        let notified_result: Result<(), tokio::time::error::Elapsed> =
            tokio::time::timeout(std::time::Duration::from_millis(100), notified).await;

        // After clear, current() must be None
        assert!(
            session.current().is_none(),
            "current() must be None after clear()"
        );

        assert!(
            notified_result.is_ok(),
            "cancel signal must be notified after clear() calls shutdown()"
        );
    }

    #[test]
    fn clear_is_safe_on_empty_session() {
        let session = WalletSession::empty();
        // Must not panic
        session.clear();
        assert!(
            session.current().is_none(),
            "current() must remain None after clear() on empty session"
        );
    }

    const INVALID_MNEMONIC: &str = "this is not a valid bip39 mnemonic phrase at all nope";

    // Test Budget: 4 behaviors x 2 = 8 unit tests (using 4)

    // Behavior 1: valid mnemonic sets current() to Some with matching address
    #[tokio::test]
    async fn init_from_mnemonic_valid_sets_current_to_some() {
        let session = WalletSession::empty();
        session
            .init_from_mnemonic(TEST_MNEMONIC, None, None)
            .await
            .expect("valid mnemonic must succeed");

        let svc = session.current();
        assert!(svc.is_some(), "current() must be Some after valid init");

        // external address 0 must match load_admin_wallet derivation
        let expected_wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest).unwrap();
        let expected_addr = expected_wallet
            .peek_address(bdk_wallet::KeychainKind::External, 0)
            .address
            .to_string();

        let svc = svc.unwrap();
        let wallet = svc.wallet.lock().await;
        let actual_addr = wallet
            .peek_address(bdk_wallet::KeychainKind::External, 0)
            .address
            .to_string();

        assert_eq!(
            actual_addr, expected_addr,
            "external address 0 must match load_admin_wallet derivation"
        );
    }

    // Behavior 2: invalid mnemonic returns InvalidMnemonic and slot stays None
    #[tokio::test]
    async fn init_from_mnemonic_invalid_mnemonic_returns_error_and_slot_stays_none() {
        use crate::infrastructure::admin_wallet::AdminWalletError;
        let session = WalletSession::empty();
        let result = session
            .init_from_mnemonic(INVALID_MNEMONIC, None, None)
            .await;

        assert!(
            matches!(result, Err(AdminWalletError::InvalidMnemonic(_))),
            "invalid mnemonic must return InvalidMnemonic, got: {:?}",
            result
        );
        assert!(
            session.current().is_none(),
            "slot must remain None after invalid mnemonic"
        );
    }

    // Behavior 3: succeeds regardless of BITCOIN_RPC_URL (pure local wallet build)
    // WalletService::new() reads BITCOIN_RPC_URL from env as a string but never connects —
    // connection only happens in sync()/fund_commit(). init_from_mnemonic never dials the node.
    #[tokio::test]
    async fn init_from_mnemonic_does_not_require_rpc() {
        // We do NOT set BITCOIN_RPC_URL to avoid env-var race with parallel tests.
        // The key assertion is that init_from_mnemonic returns Ok even though no live
        // Bitcoin node is available in the test environment.
        let session = WalletSession::empty();
        let result = session.init_from_mnemonic(TEST_MNEMONIC, None, None).await;
        assert!(
            result.is_ok(),
            "init_from_mnemonic must succeed without a live Bitcoin node: {:?}",
            result
        );
        assert!(
            session.current().is_some(),
            "current() must be Some after successful init"
        );
    }

    // Behavior 4: re-init shuts down prior service
    #[tokio::test]
    async fn reinit_shuts_down_prior_service() {
        let session = WalletSession::empty();

        session
            .init_from_mnemonic(TEST_MNEMONIC, None, None)
            .await
            .expect("first init must succeed");

        let first_svc = session.current().expect("first service must be present");
        let cancel = Arc::clone(&first_svc.cancel);
        // Register notified() BEFORE second init (notify_waiters() semantics)
        let notified = cancel.notified();

        session
            .init_from_mnemonic(TEST_MNEMONIC, None, None)
            .await
            .expect("second init must succeed");

        // The cancel signal from first service must have been notified
        let result: Result<(), tokio::time::error::Elapsed> =
            tokio::time::timeout(std::time::Duration::from_millis(50), notified).await;
        assert!(
            result.is_ok(),
            "prior service cancel must be notified on re-init"
        );
        assert!(
            session.current().is_some(),
            "current() must be Some after re-init"
        );
    }
}
