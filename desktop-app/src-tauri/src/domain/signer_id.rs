//! Admin signer identity as the ASM will key it: the 20-byte P2WPKH witness program.
//!
//! At the current ASM pin a role's signer set is a list of compressed public keys. From the v0.5
//! pin on it is a list of P2WPKH witness programs, and the Admin ID (`bc1q…`, `tb1q…`,
//! `bcrt1q…`) is that program rendered for one network. Comparing signers by program works for
//! both, and treats every rendering of one program as the same signer.

use bitcoin::hashes::Hash;
use bitcoin::{Address, CompressedPublicKey, Network, WitnessProgram, WitnessVersion};

/// The P2WPKH witness program (`hash160` of the compressed public key) of an admin signer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignerId([u8; 20]);

impl SignerId {
    /// The signer behind a hex-encoded compressed public key, in any letter case. `None` when the
    /// hex is not a valid compressed secp256k1 key, so it matches no signer.
    pub fn from_pubkey_hex(pubkey_hex: &str) -> Option<Self> {
        let bytes = hex::decode(pubkey_hex).ok()?;
        let key = CompressedPublicKey::from_slice(&bytes).ok()?;
        Some(Self(key.wpubkey_hash().to_byte_array()))
    }

    /// The Admin ID: this program as a P2WPKH address on `network` (`bc1q…`, `tb1q…`, `bcrt1q…`).
    pub fn admin_id(&self, network: Network) -> String {
        let program = WitnessProgram::new(WitnessVersion::V0, &self.0)
            .expect("a 20-byte v0 program is always a valid P2WPKH program");
        Address::from_witness_program(program, network).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generator point, a valid compressed key.
    const KEY: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    /// 2·G, a different valid compressed key.
    const OTHER_KEY: &str = "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";

    #[test]
    fn letter_case_does_not_change_the_signer() {
        assert_eq!(
            SignerId::from_pubkey_hex(KEY),
            SignerId::from_pubkey_hex(&KEY.to_uppercase())
        );
    }

    #[test]
    fn distinct_keys_are_distinct_signers() {
        assert_ne!(
            SignerId::from_pubkey_hex(KEY),
            SignerId::from_pubkey_hex(OTHER_KEY)
        );
    }

    #[test]
    fn every_network_rendering_of_the_admin_id_is_the_same_signer() {
        let signer = SignerId::from_pubkey_hex(KEY).expect("valid key");
        let key = CompressedPublicKey::from_slice(&hex::decode(KEY).unwrap()).unwrap();
        for network in [Network::Bitcoin, Network::Testnet, Network::Regtest] {
            let address = Address::p2wpkh(&key, network);
            let program = address.witness_program().expect("p2wpkh has a program");
            assert_eq!(
                program.program().as_bytes(),
                signer.0.as_slice(),
                "{network}"
            );
        }
        assert_eq!(signer.0, key.wpubkey_hash().to_byte_array());
    }

    #[test]
    fn admin_id_renders_the_program_on_the_given_network() {
        let signer = SignerId::from_pubkey_hex(KEY).expect("valid key");
        // BIP-173 test vector: the P2WPKH address of the generator point.
        assert_eq!(
            signer.admin_id(Network::Bitcoin),
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"
        );
        assert_eq!(
            signer.admin_id(Network::Regtest),
            "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080"
        );
    }

    #[test]
    fn invalid_input_matches_no_signer() {
        assert_eq!(SignerId::from_pubkey_hex("not hex"), None);
        assert_eq!(SignerId::from_pubkey_hex(&KEY[..64]), None);
        // Right length and prefix, but x is above the field prime, so it is not a curve point.
        assert_eq!(
            SignerId::from_pubkey_hex(&format!("02{}", "f".repeat(64))),
            None
        );
    }
}
