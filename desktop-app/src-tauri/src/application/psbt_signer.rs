use std::any::Any;

use bdk_wallet::bitcoin::psbt::Psbt;
use bdk_wallet::bitcoin::Network;

/// Driven port for PSBT signing — shared by mnemonic (simulated HW) and real hardware wallets.
pub(crate) trait PsbtSigner: Send + Sync {
    fn sign_psbt(&self, wallet: &mut bdk_wallet::Wallet, psbt: &mut Psbt) -> Result<(), String>;
    fn allowed_on(&self, network: Network) -> bool;
    fn kind(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
}

/// Software signer wrapping BDK wallet.sign — behaves as a simulated hardware wallet.
/// Allowed on regtest and testnet only (not mainnet).
pub(crate) struct MnemonicPsbtSigner;

impl MnemonicPsbtSigner {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for MnemonicPsbtSigner {
    fn default() -> Self {
        Self::new()
    }
}

impl PsbtSigner for MnemonicPsbtSigner {
    fn sign_psbt(&self, wallet: &mut bdk_wallet::Wallet, psbt: &mut Psbt) -> Result<(), String> {
        let signed = wallet
            .sign(psbt, bdk_wallet::SignOptions::default())
            .map_err(|e| e.to_string())?;
        if !signed {
            return Err("mnemonic wallet did not sign any inputs".to_string());
        }
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

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mnemonic_psbt_signer_allowed_on_capability_matrix() {
        let signer = MnemonicPsbtSigner::new();
        assert!(signer.allowed_on(Network::Regtest));
        assert!(signer.allowed_on(Network::Testnet));
        assert!(!signer.allowed_on(Network::Bitcoin));
    }
}
