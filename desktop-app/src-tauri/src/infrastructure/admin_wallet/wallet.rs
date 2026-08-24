use bdk_wallet::bitcoin::bip32::{DerivationPath, Fingerprint, Xpriv, Xpub};
use bdk_wallet::bitcoin::secp256k1::Secp256k1;
use bdk_wallet::bitcoin::{Address, Network};
use bip39::Mnemonic;

use crate::infrastructure::hw_wallet::hw_psbt_signer::HwDeviceType;
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
    #[error("Electrum unreachable: {message}")]
    ElectrumUnreachable { message: String },
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

/// BIP-86 account origin path segment for Admin Wallet (`86'/{coin}'/73'`), network-based.
/// Used as the default for software/watch-only callers that have no hardware device.
pub fn admin_wallet_account_origin_path(network: Network) -> &'static str {
    match network {
        Network::Bitcoin => "86'/0'/73'",
        _ => "86'/1'/73'",
    }
}

/// BIP-86 Admin Wallet account derivation path for a hardware device + network.
///
/// This is the single source of truth for the device-specific coin type: Trezor only
/// accepts/derives the BIP-86 account at coin type `0'` (mainnet-style) on every network,
/// while Ledger uses `0'` on mainnet and `1'` on test networks (matching its Bitcoin/Testnet
/// apps). The xpub export path, the descriptor origin and the verify-on-device path must all
/// agree with this, or the displayed address and HW PSBT signing diverge.
pub fn admin_wallet_account_path(device: HwDeviceType, network: Network) -> &'static str {
    match (device, network) {
        (HwDeviceType::Trezor, _) => "m/86'/0'/73'",
        (HwDeviceType::Ledger, Network::Bitcoin) => "m/86'/0'/73'",
        (HwDeviceType::Ledger, _) => "m/86'/1'/73'",
    }
}

/// Descriptor origin segment (no leading `m/`) for a hardware device + network.
pub fn admin_wallet_account_origin(device: HwDeviceType, network: Network) -> &'static str {
    admin_wallet_account_path(device, network)
        .strip_prefix("m/")
        .expect("account path always starts with m/")
}

/// The HRP the connected device's firmware renders for the Admin Wallet account.
///
/// A device shows an address under the coin it derives the account with, not under the
/// session network: Trezor holds the account at coin type `0'` on every network and so
/// renders `bc1…`, while Ledger uses `1'` off mainnet and renders `tb1…`. This table must
/// stay in lockstep with [`admin_wallet_account_path`] — the coin type is what the firmware
/// keys off — or an on-device verification value stops matching what the device shows.
pub fn device_hrp_network(device: HwDeviceType, network: Network) -> Network {
    match (device, network) {
        (HwDeviceType::Trezor, _) => Network::Bitcoin,
        (HwDeviceType::Ledger, Network::Bitcoin) => Network::Bitcoin,
        (HwDeviceType::Ledger, _) => Network::Testnet,
    }
}

/// Re-encodes `address` with the human-readable prefix of `hrp_network`, preserving the
/// witness program. Used to show a *device-accurate* verification value: a regtest receive
/// address (`bcrt1…`) is rendered as the device's firmware shows it (`tb1…` on the Testnet
/// app, `bc1…` on the Bitcoin app) so the signer compares identical strings on-device.
pub fn render_address_with_hrp(address: &Address, hrp_network: Network) -> String {
    Address::from_script(address.script_pubkey().as_script(), hrp_network)
        .map(|a| a.to_string())
        .unwrap_or_else(|_| address.to_string())
}

/// Load a BIP-86 taproot watch-only wallet from an account-level xpub string.
/// Builds tr(xpub/0/*) external and tr(xpub/1/*) internal descriptors — no private key.
/// When `master_fingerprint` is set, embeds BIP-380 origin info so HW PSBT signing can match inputs.
pub fn load_watch_only_admin_wallet(
    account_xpub: &str,
    network: Network,
    master_fingerprint: Option<u32>,
    account_origin: &str,
) -> Result<bdk_wallet::Wallet, AdminWalletError> {
    let mut xpub =
        Xpub::from_str(account_xpub).map_err(|e| AdminWalletError::Descriptor(e.to_string()))?;
    // BIP-32 key material (public key + chain code) is network-independent; only the
    // serialization version bytes encode the network. A hardware wallet may export a mainnet
    // xpub while the session runs on regtest/testnet. Reinterpret the version bytes for the
    // target network so the descriptor matches the wallet network without altering the derived
    // keys — the addresses stay identical to the same seed's wallet (D7 equivalence preserved).
    xpub.network = bdk_wallet::bitcoin::NetworkKind::from(network);
    let account_key = match master_fingerprint {
        Some(fp) => {
            let fingerprint = Fingerprint::from(fp.to_le_bytes());
            format!("[{}/{}]{}", fingerprint, account_origin, xpub)
        }
        None => xpub.to_string(),
    };
    let external_desc = format!("tr({}/0/*)", account_key);
    let internal_desc = format!("tr({}/1/*)", account_key);
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
        let result = load_watch_only_admin_wallet(
            &xpub,
            Network::Regtest,
            None,
            admin_wallet_account_origin_path(Network::Regtest),
        );
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
        let watch_wallet = load_watch_only_admin_wallet(
            TEST_ACCOUNT_XPUB,
            Network::Regtest,
            None,
            admin_wallet_account_origin_path(Network::Regtest),
        )
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
        let watch_wallet = load_watch_only_admin_wallet(
            &xpub,
            Network::Regtest,
            None,
            admin_wallet_account_origin_path(Network::Regtest),
        )
        .expect("watch-only wallet must succeed");
        assert_eq!(
            get_external_address(&mnemonic_wallet),
            get_external_address(&watch_wallet),
            "external address[0] must match between mnemonic and watch-only wallet"
        );
    }

    #[test]
    fn load_watch_only_admin_wallet_returns_error_for_malformed_xpub() {
        let result = load_watch_only_admin_wallet(
            "not-a-valid-xpub",
            Network::Regtest,
            None,
            admin_wallet_account_origin_path(Network::Regtest),
        );
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
        let watch_wallet = load_watch_only_admin_wallet(
            &mainnet_xpub,
            Network::Regtest,
            None,
            admin_wallet_account_origin_path(Network::Regtest),
        )
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

    #[test]
    fn admin_wallet_account_path_and_origin_are_device_aware() {
        // Trezor: coin 0' on every network; Ledger: coin 1' on test nets, 0' on mainnet.
        assert_eq!(
            admin_wallet_account_path(HwDeviceType::Trezor, Network::Regtest),
            "m/86'/0'/73'"
        );
        assert_eq!(
            admin_wallet_account_path(HwDeviceType::Ledger, Network::Regtest),
            "m/86'/1'/73'"
        );
        assert_eq!(
            admin_wallet_account_path(HwDeviceType::Ledger, Network::Bitcoin),
            "m/86'/0'/73'"
        );
        // The origin segment is the path without the leading `m/`.
        assert_eq!(
            admin_wallet_account_origin(HwDeviceType::Trezor, Network::Regtest),
            "86'/0'/73'"
        );
        assert_eq!(
            admin_wallet_account_origin(HwDeviceType::Ledger, Network::Regtest),
            "86'/1'/73'"
        );
    }

    #[test]
    fn device_hrp_network_follows_the_account_path_coin_type() {
        // The firmware renders an address under the coin it derived the account with, so the
        // HRP table and the account path must never drift apart: coin `0'` ⇒ mainnet HRP,
        // coin `1'` ⇒ testnet HRP. Issue #401 was exactly this pair going out of sync.
        for device in [HwDeviceType::Trezor, HwDeviceType::Ledger] {
            for network in [
                Network::Bitcoin,
                Network::Testnet,
                Network::Signet,
                Network::Regtest,
            ] {
                let path = admin_wallet_account_path(device, network);
                let expected = if path.starts_with("m/86'/0'/") {
                    Network::Bitcoin
                } else {
                    Network::Testnet
                };
                assert_eq!(
                    device_hrp_network(device, network),
                    expected,
                    "{device:?} on {network:?} derives at {path}; the rendered HRP must follow that coin type"
                );
            }
        }
    }

    #[test]
    fn watch_only_with_fingerprint_embeds_supplied_origin_without_changing_address() {
        // The origin is BIP-380 metadata for HW PSBT signing; it must NOT alter the derived
        // address (which depends only on the xpub key material + /0/index). A Trezor session
        // (origin 86'/0'/73') on regtest must still produce the same address as the mnemonic
        // wallet derived at m/86'/0'/73'.
        let xpub = derive_account_xpub_from_mnemonic(TEST_MNEMONIC, Network::Regtest)
            .expect("xpub derivation must succeed");
        let mnemonic_wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest)
            .expect("mnemonic wallet must succeed");
        let trezor_wallet = load_watch_only_admin_wallet(
            &xpub,
            Network::Regtest,
            Some(0x1234_5678),
            admin_wallet_account_origin(HwDeviceType::Trezor, Network::Regtest),
        )
        .expect("watch-only with fingerprint must succeed");
        assert_eq!(
            get_external_address(&mnemonic_wallet),
            get_external_address(&trezor_wallet),
            "embedding an origin must not change the derived address"
        );
    }

    #[test]
    fn render_address_with_hrp_swaps_prefix_preserving_witness_program() {
        // A regtest receive address (bcrt1p…) re-encoded for the device's firmware app:
        // Testnet → tb1p… (Ledger), Bitcoin → bc1p… (Trezor). Only the HRP changes; the
        // witness program (everything after the separator) stays identical.
        let wallet = load_admin_wallet(TEST_MNEMONIC, Network::Regtest)
            .expect("wallet creation must succeed");
        let addr = get_external_address(&wallet);
        let regtest = addr.to_string();
        assert!(regtest.starts_with("bcrt1p"), "precondition: {regtest}");

        let testnet = render_address_with_hrp(&addr, Network::Testnet);
        let mainnet = render_address_with_hrp(&addr, Network::Bitcoin);
        assert!(testnet.starts_with("tb1p"), "expected tb1p…, got {testnet}");
        assert!(mainnet.starts_with("bc1p"), "expected bc1p…, got {mainnet}");

        // Witness version + program are identical across HRPs; only the HRP and the trailing
        // 6-char bech32 checksum (which is computed over the HRP) differ.
        let body = |s: &str| {
            let data = s.rsplit_once('1').expect("bech32 separator").1;
            data[..data.len() - 6].to_string()
        };
        assert_eq!(body(&regtest), body(&testnet));
        assert_eq!(body(&regtest), body(&mainnet));
    }
}
