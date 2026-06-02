use crate::application::psbt_signer::MnemonicPsbtSigner;
use crate::application::wallet_service::WalletService;
use crate::infrastructure::admin_wallet::wallet::load_watch_only_admin_wallet;
use crate::infrastructure::admin_wallet::{load_admin_wallet, AdminWalletError};
use crate::infrastructure::hw_wallet::hw_psbt_signer::{HwDeviceType, HwPsbtSigner};
use std::sync::{Arc, RwLock};

/// Live session: Admin Wallet service (mnemonic not stored).
pub(crate) struct SessionState {
    pub wallet: Arc<WalletService>,
}

#[derive(Clone)]
pub struct WalletSession {
    pub(crate) inner: Arc<RwLock<Option<SessionState>>>,
}

impl WalletSession {
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
        }
    }

    fn read_slot(&self) -> Option<SessionState> {
        self.inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|s| SessionState {
                wallet: Arc::clone(&s.wallet),
            })
    }

    pub fn can_sign(&self) -> bool {
        self.read_slot()
            .map(|s| s.wallet.can_sign())
            .unwrap_or(false)
    }

    pub fn current(&self) -> Option<Arc<WalletService>> {
        self.read_slot().map(|s| s.wallet)
    }

    pub fn clear(&self) {
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = guard.take() {
            state.wallet.shutdown();
        }
    }

    fn build_session_from_mnemonic(
        mnemonic: &str,
        network: bdk_wallet::bitcoin::Network,
    ) -> Result<SessionState, AdminWalletError> {
        let wallet = load_admin_wallet(mnemonic, network)?;
        let signer = Arc::new(MnemonicPsbtSigner::new(network));
        let wallet = Arc::new(WalletService::with_signer(wallet, signer));
        Ok(SessionState { wallet })
    }

    fn build_session_from_xpub(
        account_xpub: &str,
        network: bdk_wallet::bitcoin::Network,
    ) -> Result<SessionState, AdminWalletError> {
        let wallet = load_watch_only_admin_wallet(account_xpub, network, None)?;
        // HW path: attach HwPsbtSigner when master_fingerprint is provided (slice b).
        // For now (slice a), this creates a watch-only session with no signer.
        let wallet = Arc::new(WalletService::new_watch_only(wallet));
        Ok(SessionState { wallet })
    }

    /// Initialize session from hardware wallet xpub with master fingerprint.
    /// The master fingerprint is captured at connect time (not derived from xpub).
    pub async fn init_from_xpub_with_hw(
        &self,
        account_xpub: &str,
        master_fingerprint: u32,
        device_type: HwDeviceType,
        network: Option<&str>,
    ) -> Result<(), AdminWalletError> {
        let net = parse_network(network);
        let wallet = load_watch_only_admin_wallet(account_xpub, net, Some(master_fingerprint))?;
        let signer = Arc::new(HwPsbtSigner::new(
            master_fingerprint,
            device_type,
            account_xpub.to_string(),
            net,
        ));
        let wallet = Arc::new(WalletService::with_signer(wallet, signer));
        let state = SessionState { wallet };
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(old) = guard.take() {
            old.wallet.shutdown();
        }
        *guard = Some(state);
        Ok(())
    }

    pub async fn init_from_xpub(
        &self,
        account_xpub: &str,
        network: Option<&str>,
    ) -> Result<(), AdminWalletError> {
        let net = parse_network(network);
        let state = Self::build_session_from_xpub(account_xpub, net)?;
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(old) = guard.take() {
            old.wallet.shutdown();
        }
        *guard = Some(state);
        Ok(())
    }

    pub async fn init_from_mnemonic(
        &self,
        mnemonic: &str,
        _passphrase: Option<&str>,
        network: Option<&str>,
    ) -> Result<(), AdminWalletError> {
        let net = parse_network(network);
        let state = Self::build_session_from_mnemonic(mnemonic, net)?;
        let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(old) = guard.take() {
            old.wallet.shutdown();
        }
        *guard = Some(state);
        Ok(())
    }

    /// Returns the active session wallet, or [`AdminWalletError::Disabled`] when logged out.
    pub fn current_or_fallback(&self) -> Result<Arc<WalletService>, AdminWalletError> {
        self.current().ok_or(AdminWalletError::Disabled)
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

    fn make_session_state() -> SessionState {
        WalletSession::build_session_from_mnemonic(TEST_MNEMONIC, Network::Regtest)
            .expect("wallet ok")
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
        let state = make_session_state();
        let cancel = Arc::clone(&state.wallet.cancel);

        {
            let mut guard = session.inner.write().unwrap_or_else(|e| e.into_inner());
            *guard = Some(state);
        }

        assert!(
            session.current().is_some(),
            "service must be present before clear()"
        );

        let notified = cancel.notified();

        let session_clone = session.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            session_clone.clear();
        });

        let notified_result: Result<(), tokio::time::error::Elapsed> =
            tokio::time::timeout(std::time::Duration::from_millis(100), notified).await;

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
        session.clear();
        assert!(
            session.current().is_none(),
            "current() must remain None after clear() on empty session"
        );
    }

    const INVALID_MNEMONIC: &str = "this is not a valid bip39 mnemonic phrase at all nope";

    #[tokio::test]
    async fn init_from_mnemonic_valid_sets_current_to_some() {
        let session = WalletSession::empty();
        session
            .init_from_mnemonic(TEST_MNEMONIC, None, None)
            .await
            .expect("valid mnemonic must succeed");

        let svc = session.current();
        assert!(svc.is_some(), "current() must be Some after valid init");

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

    #[tokio::test]
    async fn init_from_mnemonic_does_not_require_rpc() {
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

    #[tokio::test]
    async fn current_or_fallback_returns_session_wallet_after_init() {
        let session = WalletSession::empty();
        session
            .init_from_mnemonic(TEST_MNEMONIC, None, None)
            .await
            .expect("init must succeed");

        let expected_addr = load_admin_wallet(TEST_MNEMONIC, Network::Regtest)
            .unwrap()
            .peek_address(bdk_wallet::KeychainKind::External, 0)
            .address
            .to_string();

        let svc = session
            .current_or_fallback()
            .expect("must return Ok with active session");
        let wallet = svc.wallet.lock().await;
        let actual_addr = wallet
            .peek_address(bdk_wallet::KeychainKind::External, 0)
            .address
            .to_string();

        assert_eq!(actual_addr, expected_addr);
    }

    #[test]
    fn current_or_fallback_returns_disabled_when_no_session() {
        let session = WalletSession::empty();
        let result = session.current_or_fallback();

        assert!(
            matches!(result, Err(AdminWalletError::Disabled)),
            "must return Disabled when no session, got: {:?}",
            result.map(|_| "<WalletService>")
        );
    }

    #[tokio::test]
    async fn init_from_mnemonic_with_testnet_network_uses_testnet() {
        let session = WalletSession::empty();
        let result = session
            .init_from_mnemonic(
                "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
                None,
                Some("testnet"),
            )
            .await;
        assert!(result.is_ok(), "testnet init must succeed");
        let svc = session.current().expect("session must be Some after init");
        let wallet = svc.wallet.lock().await;
        assert_eq!(wallet.network(), bdk_wallet::bitcoin::Network::Testnet);
    }

    #[tokio::test]
    async fn init_from_mnemonic_with_mainnet_network_uses_mainnet() {
        let session = WalletSession::empty();
        let result = session
            .init_from_mnemonic(
                "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
                None,
                Some("bitcoin"),
            )
            .await;
        assert!(result.is_ok(), "mainnet init must succeed");
        let svc = session.current().expect("session must be Some after init");
        let wallet = svc.wallet.lock().await;
        assert_eq!(wallet.network(), bdk_wallet::bitcoin::Network::Bitcoin);
    }

    // ---- xpub / watch-only tests ----

    fn derive_account_xpub(mnemonic_str: &str, network: Network) -> String {
        use bdk_wallet::bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
        use bdk_wallet::bitcoin::secp256k1::Secp256k1;
        use std::str::FromStr;
        let mnemonic = bip39::Mnemonic::parse(mnemonic_str).unwrap();
        let seed = mnemonic.to_seed("");
        let secp = Secp256k1::new();
        let xpriv = Xpriv::new_master(network, &seed).unwrap();
        let path = DerivationPath::from_str("m/86h/0h/73h").unwrap();
        let account_xpriv = xpriv.derive_priv(&secp, &path).unwrap();
        Xpub::from_priv(&secp, &account_xpriv).to_string()
    }

    // Acceptance test: init_from_xpub sets a watch-only session (current() Some, can_sign() false)
    #[tokio::test]
    async fn init_from_xpub_sets_session_as_watch_only() {
        let session = WalletSession::empty();
        let xpub = derive_account_xpub(TEST_MNEMONIC, Network::Regtest);
        session
            .init_from_xpub(&xpub, None)
            .await
            .expect("valid xpub must succeed");
        assert!(
            session.current().is_some(),
            "current() must be Some after xpub init"
        );
        assert!(
            !session.can_sign(),
            "can_sign() must be false after xpub init"
        );
    }

    #[tokio::test]
    async fn init_from_xpub_malformed_returns_error_and_slot_stays_none() {
        let session = WalletSession::empty();
        let result = session.init_from_xpub("not-a-valid-xpub", None).await;
        assert!(result.is_err(), "malformed xpub must return error");
        assert!(
            session.current().is_none(),
            "slot must remain None after error"
        );
    }

    #[tokio::test]
    async fn can_sign_flips_correctly_between_mnemonic_and_xpub_init() {
        let session = WalletSession::empty();
        assert!(
            !session.can_sign(),
            "can_sign() must be false with no session"
        );

        session
            .init_from_mnemonic(TEST_MNEMONIC, None, None)
            .await
            .expect("mnemonic init must succeed");
        assert!(
            session.can_sign(),
            "can_sign() must be true after mnemonic init"
        );

        let xpub = derive_account_xpub(TEST_MNEMONIC, Network::Regtest);
        session
            .init_from_xpub(&xpub, None)
            .await
            .expect("xpub init must succeed");
        assert!(
            !session.can_sign(),
            "can_sign() must be false after xpub init"
        );
    }

    #[tokio::test]
    async fn reinit_shuts_down_prior_service() {
        let session = WalletSession::empty();

        session
            .init_from_mnemonic(TEST_MNEMONIC, None, None)
            .await
            .expect("first init must succeed");

        let first_svc = session.current().expect("first service must be present");
        let cancel = Arc::clone(&first_svc.cancel);
        let notified = cancel.notified();

        session
            .init_from_mnemonic(TEST_MNEMONIC, None, None)
            .await
            .expect("second init must succeed");

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
