//! Admin signer identity as the ASM keys it: the 20-byte P2WPKH witness program.
//!
//! A role's signer set is a list of P2WPKH witness programs, and the Admin ID (`bc1q…`, `tb1q…`,
//! `bcrt1q…`) is that program rendered for one network. Comparing signers by program treats every
//! rendering of one program as the same signer.

use std::str::FromStr;

use bitcoin::address::NetworkUnchecked;
use bitcoin::hashes::Hash;
use bitcoin::{Address, AddressType, CompressedPublicKey, Network, WitnessProgram, WitnessVersion};

/// The P2WPKH witness program (`hash160` of the compressed public key) of an admin signer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignerId([u8; 20]);

/// Why a typed Admin ID was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdminIdError {
    #[error("expected an Admin ID (P2WPKH address), not a compressed public key")]
    PublicKey,
    #[error("not a valid Bitcoin address: {0}")]
    Invalid(String),
    #[error("Admin ID is not a P2WPKH address")]
    NotP2wpkh,
    #[error("Admin ID is not for the {0} network")]
    WrongNetwork(Network),
}

impl SignerId {
    /// The signer behind a hex-encoded compressed public key, in any letter case. `None` when the
    /// hex is not a valid compressed secp256k1 key, so it matches no signer.
    pub fn from_pubkey_hex(pubkey_hex: &str) -> Option<Self> {
        let bytes = hex::decode(pubkey_hex).ok()?;
        let key = CompressedPublicKey::from_slice(&bytes).ok()?;
        Some(Self(key.wpubkey_hash().to_byte_array()))
    }

    /// The signer behind a witness program, as the ASM stores it.
    pub fn from_program(program: [u8; 20]) -> Self {
        Self(program)
    }

    /// The signer behind an Admin ID rendered on any network. `None` when it is not a P2WPKH
    /// address, so it matches no signer. For input a person typed, use [`Self::parse_admin_id`].
    pub fn from_admin_id(admin_id: &str) -> Option<Self> {
        let address = Address::<NetworkUnchecked>::from_str(admin_id.trim()).ok()?;
        program_of(address.assume_checked_ref())
    }

    /// An Admin ID a person typed, which must be a P2WPKH address on `network`. A compressed
    /// public key gets its own error, because it is the format this field used to take.
    pub fn parse_admin_id(input: &str, network: Network) -> Result<Self, AdminIdError> {
        let trimmed = input.trim();
        if Self::from_pubkey_hex(trimmed.strip_prefix("0x").unwrap_or(trimmed)).is_some() {
            return Err(AdminIdError::PublicKey);
        }
        let address = Address::<NetworkUnchecked>::from_str(trimmed)
            .map_err(|e| AdminIdError::Invalid(e.to_string()))?;
        if !address.is_valid_for_network(network) {
            return Err(AdminIdError::WrongNetwork(network));
        }
        program_of(address.assume_checked_ref()).ok_or(AdminIdError::NotP2wpkh)
    }

    /// The 20-byte witness program.
    pub fn program(&self) -> [u8; 20] {
        self.0
    }

    /// The Admin ID: this program as a P2WPKH address on `network` (`bc1q…`, `tb1q…`, `bcrt1q…`).
    pub fn admin_id(&self, network: Network) -> String {
        let program = WitnessProgram::new(WitnessVersion::V0, &self.0)
            .expect("a 20-byte v0 program is always a valid P2WPKH program");
        Address::from_witness_program(program, network).to_string()
    }
}

fn program_of(address: &Address) -> Option<SignerId> {
    if address.address_type() != Some(AddressType::P2wpkh) {
        return None;
    }
    let program: [u8; 20] = address
        .witness_program()?
        .program()
        .as_bytes()
        .try_into()
        .ok()?;
    Some(SignerId(program))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generator point, a valid compressed key.
    const KEY: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    /// 2·G, a different valid compressed key.
    const OTHER_KEY: &str = "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";
    /// BIP-173 test vectors: the P2WPKH address of the generator point.
    const MAINNET_ID: &str = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
    const REGTEST_ID: &str = "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080";

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
            assert_eq!(
                SignerId::from_admin_id(&address.to_string()),
                Some(signer),
                "{network}"
            );
        }
        assert_eq!(signer.program(), key.wpubkey_hash().to_byte_array());
        assert_eq!(SignerId::from_program(signer.program()), signer);
    }

    #[test]
    fn admin_id_renders_the_program_on_the_given_network() {
        let signer = SignerId::from_pubkey_hex(KEY).expect("valid key");
        assert_eq!(signer.admin_id(Network::Bitcoin), MAINNET_ID);
        assert_eq!(signer.admin_id(Network::Regtest), REGTEST_ID);
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
        assert_eq!(SignerId::from_admin_id(KEY), None);
        // A P2TR address is a valid address but not a signer.
        let key = CompressedPublicKey::from_slice(&hex::decode(KEY).unwrap()).unwrap();
        let secp = bitcoin::secp256k1::Secp256k1::verification_only();
        let p2tr = Address::p2tr(&secp, key.0.x_only_public_key().0, None, Network::Regtest);
        assert_eq!(SignerId::from_admin_id(&p2tr.to_string()), None);
        assert_eq!(
            SignerId::parse_admin_id(&p2tr.to_string(), Network::Regtest),
            Err(AdminIdError::NotP2wpkh)
        );
    }

    #[test]
    fn parse_admin_id_accepts_the_active_network_only() {
        let signer = SignerId::from_pubkey_hex(KEY).unwrap();
        assert_eq!(
            SignerId::parse_admin_id(&format!("  {REGTEST_ID} "), Network::Regtest),
            Ok(signer)
        );
        assert_eq!(
            SignerId::parse_admin_id(MAINNET_ID, Network::Regtest),
            Err(AdminIdError::WrongNetwork(Network::Regtest))
        );
        assert_eq!(
            SignerId::parse_admin_id(MAINNET_ID, Network::Bitcoin),
            Ok(signer)
        );
    }

    #[test]
    fn parse_admin_id_names_a_pasted_public_key() {
        assert_eq!(
            SignerId::parse_admin_id(KEY, Network::Regtest),
            Err(AdminIdError::PublicKey)
        );
        assert_eq!(
            SignerId::parse_admin_id(&format!("0x{KEY}"), Network::Regtest),
            Err(AdminIdError::PublicKey)
        );
        assert!(matches!(
            SignerId::parse_admin_id("bcrt1qnotanaddress", Network::Regtest),
            Err(AdminIdError::Invalid(_))
        ));
    }
}
