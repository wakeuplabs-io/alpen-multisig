//! Wallet service — application service (driving port).
//!
//! `WalletService` holds an optional signer and reports signing capability
//! based on whether the attached signer is allowed on the wallet's network.

use std::sync::Arc;

use bitcoin::Network;

use crate::application::psbt_signer::PsbtSigner;

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
}
