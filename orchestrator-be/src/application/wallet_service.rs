//! Wallet service — application service (driving port).
//!
//! `WalletService` holds an optional signer and reports signing capability
//! based on whether the attached signer is allowed on the wallet's network.

use std::sync::Arc;

use bitcoin::Network;

use crate::application::psbt_signer::PsbtSigner;
use crate::infrastructure::admin_wallet::wallet::AdminWalletError;

/// Application service that manages signing capability for a wallet.
pub(crate) struct WalletService {
    #[allow(dead_code)]
    network: Network,
    signer: Option<Arc<dyn PsbtSigner>>,
}

#[allow(dead_code)]
impl WalletService {
    pub(crate) fn new(network: Network, signer: Option<Arc<dyn PsbtSigner>>) -> Self {
        Self { network, signer }
    }

    /// Whether the session has signing capability: signer present AND
    /// the signer is allowed on this wallet's network.
    pub(crate) fn can_sign(&self) -> bool {
        self.signer
            .as_ref()
            .map(|s| s.allowed_on(self.network))
            .unwrap_or(false)
    }

    /// Build a signed commit. Returns ReadOnly if no signer is attached.
    pub(crate) fn build_signed_commit(
        &self,
        _psbt: &mut bitcoin::psbt::Psbt,
    ) -> Result<(), AdminWalletError> {
        self.signer.as_ref().ok_or(AdminWalletError::ReadOnly)?;
        // TODO: actual signing in future steps
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::application::psbt_signer::MnemonicPsbtSigner;

    #[test]
    fn test_wallet_service_can_sign_reflects_network_capability() {
        // Signer present AND allowed on network → can_sign = true
        let signer = Arc::new(MnemonicPsbtSigner::new(Network::Regtest));
        let svc = WalletService::new(Network::Regtest, Some(signer));
        assert!(
            svc.can_sign(),
            "signer present + allowed on regtest → can_sign=true"
        );

        // Signer present but NOT allowed on network → can_sign = false
        let signer = Arc::new(MnemonicPsbtSigner::new(Network::Regtest));
        let svc = WalletService::new(Network::Bitcoin, Some(signer));
        assert!(
            !svc.can_sign(),
            "signer present but rejected on mainnet → can_sign=false"
        );

        // No signer → can_sign = false
        let svc: WalletService = WalletService::new(Network::Regtest, None);
        assert!(!svc.can_sign(), "no signer → can_sign=false");
    }

    #[test]
    fn test_build_signed_commit_no_signer_returns_readonly() {
        // HW session present but no signer attached → ReadOnly error
        let svc: WalletService = WalletService::new(Network::Regtest, None);
        let tx = bitcoin::Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::ZERO,
            input: vec![],
            output: vec![],
        };
        let mut psbt = bitcoin::psbt::Psbt::from_unsigned_tx(tx).unwrap();
        let result = svc.build_signed_commit(&mut psbt);
        assert!(
            matches!(result, Err(AdminWalletError::ReadOnly)),
            "no signer attached → build_signed_commit returns ReadOnly"
        );
    }
}
