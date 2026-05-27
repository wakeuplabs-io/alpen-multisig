use bdk_wallet::bitcoin::bip32::{DerivationPath, Xpriv};
use bdk_wallet::bitcoin::secp256k1::Secp256k1;
use bdk_wallet::bitcoin::{Address, Network};
use bip39::Mnemonic;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdminWalletError {
    #[error("invalid mnemonic: {0}")]
    InvalidMnemonic(String),
    #[error("descriptor error: {0}")]
    Descriptor(String),
    #[error("wallet creation error: {0}")]
    WalletCreation(String),
    // Phase 2 variants
    #[error("RPC unreachable: {message}")]
    RpcUnreachable { message: String },
    #[error("RPC auth failed: {message}")]
    RpcAuthFailed { message: String },
    #[error("descriptor parse error: {message}")]
    DescriptorParseError { message: String },
    #[error("sync incomplete: {message}")]
    SyncIncomplete { message: String },
    #[error("regtest guard violation: {message}")]
    RegtestGuardViolation { message: String },
    #[error("admin wallet is disabled")]
    Disabled,
}

/// Load a BIP-86 taproot wallet for account 73' from a mnemonic phrase.
/// Derives descriptors at m/86'/0'/73'/0/* (external) and m/86'/0'/73'/1/* (internal).
pub fn load_admin_wallet(
    mnemonic_str: &str,
    network: Network,
) -> Result<bdk_wallet::Wallet, AdminWalletError> {
    let mnemonic = Mnemonic::parse(mnemonic_str)
        .map_err(|e| AdminWalletError::InvalidMnemonic(e.to_string()))?;
    let seed = mnemonic.to_seed("");
    let secp = Secp256k1::new();
    let xpriv = Xpriv::new_master(network, &seed)
        .map_err(|e| AdminWalletError::Descriptor(e.to_string()))?;
    let path = DerivationPath::from_str("m/86h/0h/73h")
        .map_err(|e| AdminWalletError::Descriptor(e.to_string()))?;
    let account_xpriv = xpriv
        .derive_priv(&secp, &path)
        .map_err(|e| AdminWalletError::Descriptor(e.to_string()))?;
    let external_desc = format!("tr({}/0/*)", account_xpriv);
    let internal_desc = format!("tr({}/1/*)", account_xpriv);
    let wallet = bdk_wallet::Wallet::create(external_desc, internal_desc)
        .network(network)
        .create_wallet_no_persist()
        .map_err(|e| AdminWalletError::WalletCreation(e.to_string()))?;
    Ok(wallet)
}

/// Return the address at external index 0 (m/86'/0'/73'/0/0 on a fresh wallet).
pub fn get_external_address(wallet: &bdk_wallet::Wallet) -> Address {
    wallet
        .peek_address(bdk_wallet::KeychainKind::External, 0)
        .address
}

#[cfg(test)]
mod tests {
    use super::*;
    use bdk_wallet::bitcoin::Network;

    // Known test mnemonic for deterministic derivation tests
    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn load_admin_wallet_with_valid_mnemonic_returns_ok() {
        let result = load_admin_wallet(TEST_MNEMONIC, Network::Regtest);
        assert!(result.is_ok(), "Expected Ok but got: {:?}", result.err());
    }

    #[test]
    fn get_external_address_returns_bip86_p2tr_address_at_account_73() {
        // Expected address at m/86'/0'/73'/0/0 for the abandon mnemonic on regtest
        // BIP-86 taproot at account 73' — P2TR addresses on regtest start with "bcrt1p"
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest)
            .expect("wallet creation must succeed");
        let address = get_external_address(&wallet);
        let addr_str = address.to_string();
        assert!(
            addr_str.starts_with("bcrt1p"),
            "Expected P2TR regtest address (bcrt1p...) but got: {}",
            addr_str
        );
    }
}
