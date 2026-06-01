//! Wallet session management — application service (driving port).
//!
//! `WalletSession` owns the lifecycle of a signer attachment. The session is
//! created via `init_from_mnemonic` which attaches a `MnemonicPsbtSigner`,
//! or via `init_from_xpub` which attaches an `HwPsbtSigner`.

use std::sync::Arc;

use bitcoin::Network;

use crate::application::psbt_signer::{MnemonicPsbtSigner, PsbtSigner};
use crate::error::AppError;
use crate::infrastructure::hw_wallet::hw_psbt_signer::HwPsbtSigner;

/// A wallet session that holds an optional signer.
pub(crate) struct WalletSession {
    #[allow(dead_code)]
    network: Network,
    signer: Option<Arc<dyn PsbtSigner>>,
}

#[allow(dead_code)]
impl WalletSession {
    /// Create a new session initialized with a mnemonic-derived signer.
    pub(crate) fn init_from_mnemonic(network: Network, mnemonic: &str) -> Result<Self, AppError> {
        let signer = Arc::new(MnemonicPsbtSigner::new(network, mnemonic)?);
        Ok(Self {
            network,
            signer: Some(signer),
        })
    }

    /// Create a new session initialized with a hardware wallet signer.
    ///
    /// The `master_fingerprint` is captured at connect time — NOT derived from
    /// the xpub's parent_fingerprint.
    pub(crate) fn init_from_xpub(
        network: Network,
        account_xpub: &str,
        master_fingerprint: u32,
    ) -> Result<Self, AppError> {
        let signer = Arc::new(HwPsbtSigner::new(
            network,
            account_xpub,
            master_fingerprint,
        )?);
        Ok(Self {
            network,
            signer: Some(signer),
        })
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

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn test_init_from_mnemonic_attaches_mnemonic_signer() {
        let session = WalletSession::init_from_mnemonic(Network::Regtest, TEST_MNEMONIC)
            .expect("session must be created");

        assert!(session.has_signer());
        assert!(session.signer_allowed_on_network());
    }

    #[test]
    fn test_init_from_xpub_attaches_hw_signer() {
        let session = WalletSession::init_from_xpub(
            Network::Regtest,
            "tpubD6NzVbkrYhZ4X8L36T1DKRzVJQKJH7YbF3xGqVz5k3Z9w8R7T6Y5X4W3V2U1S0",
            0x12345678,
        )
        .expect("session must be created");

        assert!(session.has_signer());
    }
}
