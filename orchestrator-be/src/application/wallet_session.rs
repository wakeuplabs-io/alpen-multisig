//! Wallet session management — application service (driving port).
//!
//! `WalletSession` owns the lifecycle of a signer attachment. The session is
//! created via `init_from_mnemonic` which attaches a `MnemonicPsbtSigner`.

use std::sync::Arc;

use bitcoin::Network;

use crate::application::psbt_signer::{MnemonicPsbtSigner, PsbtSigner};

/// A wallet session that holds an optional signer.
pub(crate) struct WalletSession {
    #[allow(dead_code)]
    network: Network,
    signer: Option<Arc<dyn PsbtSigner>>,
}

#[allow(dead_code)]
impl WalletSession {
    /// Create a new session initialized with a mnemonic-derived signer.
    pub(crate) fn init_from_mnemonic(network: Network) -> Self {
        let signer = Arc::new(MnemonicPsbtSigner::new(network));
        Self {
            network,
            signer: Some(signer),
        }
    }

    /// Whether the session has signing capability.
    pub(crate) fn has_signer(&self) -> bool {
        self.signer.is_some()
    }

    /// Whether the attached signer is allowed on the session's network.
    pub(crate) fn signer_allowed_on_network(&self) -> bool {
        self.signer
            .as_ref()
            .map(|s| s.allowed_on(self.network))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_from_mnemonic_attaches_mnemonic_signer() {
        let session = WalletSession::init_from_mnemonic(Network::Regtest);

        assert!(session.has_signer());
        assert!(session.signer_allowed_on_network());
    }
}
