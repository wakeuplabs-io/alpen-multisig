//! PSBT signing port and implementations.
//!
//! `PsbtSigner` is a driven port — the application layer delegates PSBT signing
//! to whichever implementor is configured (mnemonic for test networks, hardware
//! signer for all networks in Phase 04).

use bitcoin::Network;

/// Driven port: signs PSBTs and declares which networks it is allowed on.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) trait PsbtSigner: Send + Sync {
    /// Sign a PSBT in-place using the signer's key material.
    fn sign_psbt(
        &self,
        wallet: &mut bdk_wallet::Wallet,
        psbt: &mut bitcoin::psbt::Psbt,
    ) -> Result<(), crate::error::AppError>;

    /// Whether this signer is allowed to operate on the given network.
    fn allowed_on(&self, network: Network) -> bool;
}

/// Software signer using a BIP-39 mnemonic (simulated hardware for test networks).
///
/// Allowed on: regtest, testnet (and their signet/testnet4 variants).
/// Rejected on: bitcoin mainnet.
#[allow(dead_code)]
pub(crate) struct MnemonicPsbtSigner {
    network: Network,
    wallet: bdk_wallet::Wallet,
}

#[allow(dead_code)]
impl MnemonicPsbtSigner {
    pub(crate) fn new(network: Network, mnemonic: &str) -> Result<Self, crate::error::AppError> {
        use bdk_wallet::bitcoin::bip32::{DerivationPath, Xpriv};
        use bdk_wallet::bitcoin::secp256k1::Secp256k1;
        use bip39::Mnemonic;
        use std::str::FromStr;

        let mnemonic = Mnemonic::parse(mnemonic)
            .map_err(|e| crate::error::AppError::BadRequest(format!("invalid mnemonic: {e}")))?;
        let seed = mnemonic.to_seed("");
        let secp = Secp256k1::new();
        let xpriv = Xpriv::new_master(network, &seed)
            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!("master key: {e}")))?;
        let path = DerivationPath::from_str("m/86'/0'/73'")
            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!("path: {e}")))?;
        let account_xpriv = xpriv
            .derive_priv(&secp, &path)
            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!("derive: {e}")))?;

        let external_desc = format!("tr({}/0/*)", account_xpriv);
        let internal_desc = format!("tr({}/1/*)", account_xpriv);

        let wallet = bdk_wallet::Wallet::create(external_desc, internal_desc)
            .network(network)
            .create_wallet_no_persist()
            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!("wallet: {e}")))?;

        Ok(Self { network, wallet })
    }

    /// Access the underlying BDK wallet for funding operations.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn wallet(&self) -> &bdk_wallet::Wallet {
        &self.wallet
    }
}

impl PsbtSigner for MnemonicPsbtSigner {
    fn sign_psbt(
        &self,
        wallet: &mut bdk_wallet::Wallet,
        psbt: &mut bitcoin::psbt::Psbt,
    ) -> Result<(), crate::error::AppError> {
        wallet
            .sign(psbt, bdk_wallet::SignOptions::default())
            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!("sign_psbt: {e}")))?;
        Ok(())
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

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn test_psbt_signer_allowed_on_capability_matrix() {
        let signer = MnemonicPsbtSigner::new(Network::Regtest, TEST_MNEMONIC).unwrap();

        // Mnemonic signer permits test networks
        assert!(signer.allowed_on(Network::Regtest));
        assert!(signer.allowed_on(Network::Testnet));

        // Mnemonic signer rejects mainnet
        assert!(!signer.allowed_on(Network::Bitcoin));
    }
}
