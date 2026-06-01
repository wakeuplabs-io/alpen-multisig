use bdk_wallet::bitcoin::bip32::{DerivationPath, Xpriv, Xpub};
use bdk_wallet::bitcoin::secp256k1::Secp256k1;
use bdk_wallet::bitcoin::{Address, Network};
use bip39::Mnemonic;
use serde::Serialize;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
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
    #[error("admin wallet is watch-only; hardware wallet required to sign")]
    ReadOnly,
    #[error("signer not allowed on this network")]
    SignerNotAllowedOnNetwork,
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

/// Load a BIP-86 taproot watch-only wallet from an account-level xpub string.
/// Builds tr(xpub/0/*) external and tr(xpub/1/*) internal descriptors — no private key.
pub fn load_watch_only_admin_wallet(
    account_xpub: &str,
    network: Network,
) -> Result<bdk_wallet::Wallet, AdminWalletError> {
    let mut xpub =
        Xpub::from_str(account_xpub).map_err(|e| AdminWalletError::Descriptor(e.to_string()))?;
    // BIP-32 key material (public key + chain code) is network-independent; only the
    // serialization version bytes encode the network. A hardware wallet may export a mainnet
    // xpub while the session runs on regtest/testnet. Reinterpret the version bytes for the
    // target network so the descriptor matches the wallet network without altering the derived
    // keys — the addresses stay identical to the same seed's wallet (D7 equivalence preserved).
    xpub.network = bdk_wallet::bitcoin::NetworkKind::from(network);
    let external_desc = format!("tr({}/0/*)", xpub);
    let internal_desc = format!("tr({}/1/*)", xpub);
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

    #[test]
    fn load_watch_only_admin_wallet_returns_ok_for_valid_xpub() {
        // Acceptance: calling through the driving port (public fn) with a valid xpub returns Ok
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest)
            .expect("mnemonic wallet must succeed");
        let xpub = derive_account_xpub_from_mnemonic(TEST_MNEMONIC, Network::Regtest)
            .expect("xpub derivation must succeed");
        let result = load_watch_only_admin_wallet(&xpub, Network::Regtest);
        assert!(result.is_ok(), "Expected Ok but got: {:?}", result.err());
        let _ = wallet;
    }

    fn derive_account_xpub_from_mnemonic(
        mnemonic_str: &str,
        network: Network,
    ) -> Result<String, AdminWalletError> {
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
        let account_xpub = Xpub::from_priv(&secp, &account_xpriv);
        Ok(account_xpub.to_string())
    }

    // Known test mnemonic for deterministic derivation tests
    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    // Account-level xpub for TEST_MNEMONIC at m/86'/0'/73' on Regtest (tpub prefix).
    // Derived once and pinned as a constant to serve as the D7 regression anchor:
    // same seed must produce identical addresses whether loaded via mnemonic or xpub.
    const TEST_ACCOUNT_XPUB: &str = "tpubDC3pD7UZXnsjtxnhe3cD8eQUqW9jukHzXmdkpGP55pUAsjhmU9bUmwP8uvQWzhLMr2Fuqd947YAEnhQ9R95H84hrhhApsaWeVLU76jT7Bf1";

    #[test]
    fn watch_only_and_mnemonic_wallet_produce_same_external_address() {
        // D7 anchor: external[0] from watch-only (TEST_ACCOUNT_XPUB) must equal
        // external[0] from mnemonic wallet (TEST_MNEMONIC) — same seed, Regtest network.
        let mnemonic_wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest)
            .expect("mnemonic wallet must succeed");
        let watch_wallet = load_watch_only_admin_wallet(TEST_ACCOUNT_XPUB, Network::Regtest)
            .expect("watch-only wallet must succeed");
        assert_eq!(
            get_external_address(&mnemonic_wallet),
            get_external_address(&watch_wallet),
            "external address[0] must match between mnemonic and watch-only wallet (D7 invariant)"
        );
    }

    #[test]
    fn watch_only_wallet_external_address_matches_mnemonic_wallet_address() {
        // D7 anchor: external[0] of watch-only must equal external[0] of mnemonic wallet
        let mnemonic_wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest)
            .expect("mnemonic wallet must succeed");
        let xpub = derive_account_xpub_from_mnemonic(TEST_MNEMONIC, Network::Regtest)
            .expect("xpub derivation must succeed");
        let watch_wallet = load_watch_only_admin_wallet(&xpub, Network::Regtest)
            .expect("watch-only wallet must succeed");
        assert_eq!(
            get_external_address(&mnemonic_wallet),
            get_external_address(&watch_wallet),
            "external address[0] must match between mnemonic and watch-only wallet"
        );
    }

    #[test]
    fn load_watch_only_admin_wallet_returns_error_for_malformed_xpub() {
        let result = load_watch_only_admin_wallet("not-a-valid-xpub", Network::Regtest);
        assert!(result.is_err(), "Expected Err for malformed xpub");
    }

    #[test]
    fn load_watch_only_admin_wallet_accepts_mainnet_xpub_on_regtest() {
        // Regression: a hardware wallet (e.g. Trezor) may export a mainnet xpub (`xpub...`)
        // while the session runs on regtest. The wallet must still build, and derive the SAME
        // address as the regtest mnemonic wallet (version bytes differ, key material does not).
        let mainnet_xpub = derive_account_xpub_from_mnemonic(TEST_MNEMONIC, Network::Bitcoin)
            .expect("mainnet xpub derivation must succeed");
        assert!(
            mainnet_xpub.starts_with("xpub"),
            "precondition: expected a mainnet xpub, got: {mainnet_xpub}"
        );
        let watch_wallet = load_watch_only_admin_wallet(&mainnet_xpub, Network::Regtest)
            .expect("watch-only must accept a mainnet xpub on regtest");
        let mnemonic_wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest)
            .expect("mnemonic wallet must succeed");
        assert_eq!(
            get_external_address(&mnemonic_wallet),
            get_external_address(&watch_wallet),
            "address must match regardless of the source xpub's network version bytes"
        );
    }

    #[test]
    fn admin_wallet_error_readonly_variant_exists() {
        let err = AdminWalletError::ReadOnly;
        assert_eq!(
            err.to_string(),
            "admin wallet is watch-only; hardware wallet required to sign"
        );
    }

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
