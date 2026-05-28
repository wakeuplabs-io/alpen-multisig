use bdk_wallet::bitcoin::bip32::{DerivationPath, Xpriv};
use bdk_wallet::bitcoin::Network;
use bip39::Mnemonic;
use bitcoin::key::UntweakedKeypair;
use bitcoin::secp256k1::SECP256K1;
use std::str::FromStr;

use super::wallet::AdminWalletError;

/// Derive the SPS-50 commit/reveal internal keypair from the Admin Wallet mnemonic
/// at BIP-86 path m/86'/0'/73'/2/0.
///
/// This path is reserved for the commit/reveal internal key, distinct from the
/// Admin Wallet's external (chain 0) and internal (chain 1) address chains.
pub(crate) fn derive_commit_reveal_keypair(
    mnemonic: &str,
    network: Network,
) -> Result<UntweakedKeypair, AdminWalletError> {
    let mnemonic =
        Mnemonic::parse(mnemonic).map_err(|e| AdminWalletError::InvalidMnemonic(e.to_string()))?;
    let seed = mnemonic.to_seed("");
    let xpriv = Xpriv::new_master(network, &seed)
        .map_err(|e| AdminWalletError::Descriptor(e.to_string()))?;
    let path = DerivationPath::from_str("m/86h/0h/73h/2/0")
        .map_err(|e| AdminWalletError::Descriptor(e.to_string()))?;
    let child_xpriv = xpriv
        .derive_priv(SECP256K1, &path)
        .map_err(|e| AdminWalletError::Descriptor(e.to_string()))?;
    let sk = child_xpriv.private_key;
    let keypair = UntweakedKeypair::from_secret_key(SECP256K1, &sk);
    Ok(keypair)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bdk_wallet::bitcoin::Network;

    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn derive_commit_reveal_keypair_happy_path_returns_ok() {
        let result = derive_commit_reveal_keypair(TEST_MNEMONIC, Network::Regtest);
        assert!(result.is_ok(), "expected Ok but got: {:?}", result.err());
    }

    #[test]
    fn derive_commit_reveal_keypair_pinned_xonly_pubkey() {
        use bitcoin::secp256k1::XOnlyPublicKey;

        const PINNED_X_ONLY_HEX: &str =
            "c593affb7a5ddc102af2dfb91f3ee2cd7fca752273056a6766fb13fbe33b78e8";

        let keypair = derive_commit_reveal_keypair(TEST_MNEMONIC, Network::Regtest)
            .expect("derivation must succeed");
        let (xonly, _) = XOnlyPublicKey::from_keypair(&keypair);
        let actual_hex = hex::encode(xonly.serialize());
        assert_eq!(
            actual_hex, PINNED_X_ONLY_HEX,
            "XOnlyPublicKey must match pinned constant"
        );
    }

    #[test]
    fn derive_commit_reveal_keypair_empty_mnemonic_returns_invalid_mnemonic() {
        let result = derive_commit_reveal_keypair("", Network::Regtest);
        assert!(
            matches!(
                result,
                Err(
                    crate::infrastructure::admin_wallet::wallet::AdminWalletError::InvalidMnemonic(
                        _
                    )
                )
            ),
            "expected InvalidMnemonic, got: {:?}",
            result
        );
    }

    #[test]
    fn derive_commit_reveal_keypair_malformed_mnemonic_returns_invalid_mnemonic() {
        let result =
            derive_commit_reveal_keypair("not a valid bip39 phrase at all xyz", Network::Regtest);
        assert!(
            matches!(
                result,
                Err(
                    crate::infrastructure::admin_wallet::wallet::AdminWalletError::InvalidMnemonic(
                        _
                    )
                )
            ),
            "expected InvalidMnemonic, got: {:?}",
            result
        );
    }

    #[test]
    fn derive_commit_reveal_keypair_same_secret_key_across_networks() {
        let kp_regtest = derive_commit_reveal_keypair(TEST_MNEMONIC, Network::Regtest)
            .expect("regtest derivation must succeed");
        let kp_mainnet = derive_commit_reveal_keypair(TEST_MNEMONIC, Network::Bitcoin)
            .expect("mainnet derivation must succeed");

        assert_eq!(
            kp_regtest.secret_key().secret_bytes(),
            kp_mainnet.secret_key().secret_bytes(),
            "secret key bytes must be network-agnostic"
        );
    }

    /// Integration regression hook (spec case 10): commit address is deterministic from
    /// TEST_MNEMONIC + fixed payload. Any BDK/bitcoin crate bump or path change trips this.
    #[test]
    fn integration_commit_address_pinned_from_mnemonic_and_fixed_payload() {
        use crate::infrastructure::broadcast_tx::derive_commit_address;

        // Payload must be >= 126 bytes (EnvelopeScriptBuilder minimum).
        // This is a fixed 128-byte literal — never change it; changing it re-pins the address.
        const FIXED_PAYLOAD: &[u8] = &[
            0x61, 0x6c, 0x70, 0x65, 0x6e, 0x2d, 0x70, 0x68, 0x61, 0x73, 0x65, 0x2d, 0x33, 0x2e,
            0x35, 0x2d, 0x72, 0x65, 0x67, 0x72, 0x65, 0x73, 0x73, 0x69, 0x6f, 0x6e, 0x2d, 0x70,
            0x61, 0x79, 0x6c, 0x6f, 0x61, 0x64, 0x2d, 0x66, 0x6f, 0x72, 0x2d, 0x63, 0x6f, 0x6d,
            0x6d, 0x69, 0x74, 0x2d, 0x61, 0x64, 0x64, 0x72, 0x65, 0x73, 0x73, 0x2d, 0x70, 0x69,
            0x6e, 0x6e, 0x69, 0x6e, 0x67, 0x2d, 0x74, 0x65, 0x73, 0x74, 0x2d, 0x61, 0x6c, 0x70,
            0x65, 0x6e, 0x2d, 0x6d, 0x75, 0x6c, 0x74, 0x69, 0x73, 0x69, 0x67, 0x2d, 0x70, 0x68,
            0x61, 0x73, 0x65, 0x2d, 0x33, 0x70, 0x6f, 0x69, 0x6e, 0x74, 0x35, 0x2d, 0x72, 0x65,
            0x67, 0x72, 0x65, 0x73, 0x73, 0x69, 0x6f, 0x6e, 0x2d, 0x68, 0x6f, 0x6f, 0x6b, 0x2d,
            0x64, 0x6f, 0x2d, 0x6e, 0x6f, 0x74, 0x2d, 0x6d, 0x6f, 0x64, 0x69, 0x66, 0x79, 0x00,
            0x01, 0x02,
        ];
        const PINNED_COMMIT_ADDRESS: &str =
            "bcrt1pp0tth43gk66y5tmzh02a8qdlk4aq4jjq9xcq2cg8mg7d0g22r0jqnmw3m7";

        let keypair = derive_commit_reveal_keypair(TEST_MNEMONIC, Network::Regtest)
            .expect("derivation must succeed");
        let (address, _, _) =
            derive_commit_address(&keypair, FIXED_PAYLOAD, bitcoin::Network::Regtest)
                .expect("derive_commit_address must succeed");
        let actual = address.to_string();

        assert_eq!(
            actual, PINNED_COMMIT_ADDRESS,
            "Pinned commit address must match. Actual: {actual}"
        );
    }

    #[test]
    fn derive_commit_reveal_keypair_descriptor_error_unreachable_for_canonical_fixture() {
        for _ in 0..100 {
            let result = derive_commit_reveal_keypair(TEST_MNEMONIC, Network::Regtest);
            assert!(
                !matches!(
                    result,
                    Err(
                        crate::infrastructure::admin_wallet::wallet::AdminWalletError::Descriptor(
                            _
                        )
                    )
                ),
                "Descriptor error must never occur for the canonical test mnemonic"
            );
        }
    }
}
