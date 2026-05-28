use crate::application::wallet_service::WalletService;
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

    #[test]
    fn clear_removes_service_and_calls_shutdown() {
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

        session.clear();

        // After clear, current() must be None
        assert!(
            session.current().is_none(),
            "current() must be None after clear()"
        );

        // shutdown() must have been called — cancel signal must be notified
        let rt = tokio::runtime::Runtime::new().unwrap();
        let notified: Result<(), tokio::time::error::Elapsed> = rt.block_on(async {
            tokio::time::timeout(std::time::Duration::from_millis(50), cancel.notified()).await
        });
        assert!(
            notified.is_ok(),
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
}
