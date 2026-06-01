use bdk_wallet::bitcoin::psbt::Psbt;
use bdk_wallet::bitcoin::Network;

/// Driven port for PSBT signing — shared by mnemonic (simulated HW) and real hardware wallets.
#[allow(dead_code)]
pub(crate) trait PsbtSigner: Send + Sync {
    fn sign_psbt(&self, psbt: &mut Psbt) -> Result<(), String>;
    fn allowed_on(&self, network: Network) -> bool;
    fn kind(&self) -> &str;
}

/// Software signer wrapping BDK wallet.sign — behaves as a simulated hardware wallet.
/// Allowed on regtest and testnet only (not mainnet).
pub(crate) struct MnemonicPsbtSigner {
    #[allow(dead_code)]
    network: Network,
}

impl MnemonicPsbtSigner {
    pub(crate) fn new(network: Network) -> Self {
        Self { network }
    }
}

impl PsbtSigner for MnemonicPsbtSigner {
    fn sign_psbt(&self, _psbt: &mut Psbt) -> Result<(), String> {
        // Signing is done by the caller (WalletService) via wallet.sign().
        // This method is a no-op placeholder for the port interface.
        Ok(())
    }

    fn allowed_on(&self, network: Network) -> bool {
        match network {
            Network::Testnet | Network::Regtest | Network::Signet | Network::Testnet4 => true,
            Network::Bitcoin => false,
        }
    }

    fn kind(&self) -> &str {
        "mnemonic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mnemonic_psbt_signer_allowed_on_capability_matrix() {
        let signer = MnemonicPsbtSigner::new(Network::Regtest);
        assert!(signer.allowed_on(Network::Regtest));
        assert!(signer.allowed_on(Network::Testnet));
        assert!(!signer.allowed_on(Network::Bitcoin));
    }
}
