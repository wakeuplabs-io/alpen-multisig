//! PSBT signing port and implementations.
//!
//! `PsbtSigner` is a driven port — the application layer delegates PSBT signing
//! to whichever implementor is configured (mnemonic for test networks, hardware
//! signer for all networks in Phase 04).

use bitcoin::Network;

/// Driven port: signs PSBTs and declares which networks it is allowed on.
#[allow(dead_code)]
pub(crate) trait PsbtSigner: Send + Sync {
    /// Sign a PSBT in-place.
    fn sign_psbt(&self, psbt: &mut bitcoin::psbt::Psbt) -> Result<(), crate::error::AppError>;

    /// Whether this signer is allowed to operate on the given network.
    fn allowed_on(&self, network: Network) -> bool;
}

/// Software signer using a BIP-39 mnemonic (simulated hardware for test networks).
///
/// Allowed on: regtest, testnet (and their signet/testnet4 variants).
/// Rejected on: bitcoin mainnet.
#[allow(dead_code)]
pub(crate) struct MnemonicPsbtSigner {
    #[allow(dead_code)]
    network: Network,
}

#[allow(dead_code)]
impl MnemonicPsbtSigner {
    pub(crate) fn new(network: Network) -> Self {
        Self { network }
    }
}

impl PsbtSigner for MnemonicPsbtSigner {
    fn sign_psbt(&self, _psbt: &mut bitcoin::psbt::Psbt) -> Result<(), crate::error::AppError> {
        todo!("sign_psbt not yet implemented — driven by future steps")
    }

    fn allowed_on(&self, network: Network) -> bool {
        match network {
            Network::Bitcoin => false,
            Network::Testnet | Network::Signet | Network::Regtest | Network::Testnet4 => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psbt_signer_allowed_on_capability_matrix() {
        let signer = MnemonicPsbtSigner::new(Network::Regtest);

        // Mnemonic signer permits test networks
        assert!(signer.allowed_on(Network::Regtest));
        assert!(signer.allowed_on(Network::Testnet));

        // Mnemonic signer rejects mainnet
        assert!(!signer.allowed_on(Network::Bitcoin));
    }
}
