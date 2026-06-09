//! Governance actions that a signer can propose — client-side domain.
//!
//! These types mirror (a subset of) the protocol's `MultisigAction` without depending
//! on Strata crates. Translation to/from the canonical SSZ form lives in
//! `crate::infrastructure::action_codec`.

use std::num::NonZeroU8;

use crate::domain::authority::Authority;

/// An x-only (even-parity) secp256k1 public key (32 bytes, no 02/03 prefix).
///
/// Used for bridge operator keys. The ASM models these as `EvenPublicKey` (32 bytes)
/// while signer-set keys use `CompressedPublicKey` (33 bytes) — keeping them distinct
/// prevents codec errors from being silently swapped.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EvenPubKey([u8; 32]);

/// Failure to construct an `EvenPubKey`.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EvenPubKeyError {
    #[error("invalid hex: {0}")]
    Hex(String),
    #[error("expected 32 bytes, got {0}")]
    WrongLength(usize),
}

impl EvenPubKey {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn from_hex(s: &str) -> Result<Self, EvenPubKeyError> {
        let bytes = hex::decode(s).map_err(|e| EvenPubKeyError::Hex(e.to_string()))?;
        let len = bytes.len();
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| EvenPubKeyError::WrongLength(len))?;
        Ok(Self(arr))
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Update to the bridge operator set.
///
/// `add_members` are x-only public keys of operators to add.
/// `remove_members` are operator indices (u32) to remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorSetUpdate {
    pub add_members: Vec<EvenPubKey>,
    pub remove_members: Vec<u32>,
}

/// A compressed secp256k1 public key (33 bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompressedPubKey([u8; 33]);

/// Failure to construct a `CompressedPubKey`.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PubKeyError {
    #[error("invalid hex: {0}")]
    Hex(String),
    #[error("expected 33 bytes, got {0}")]
    WrongLength(usize),
}

impl CompressedPubKey {
    /// Wraps a 33-byte array.
    pub fn new(bytes: [u8; 33]) -> Self {
        Self(bytes)
    }

    /// Parses a hex-encoded compressed public key.
    pub fn from_hex(s: &str) -> Result<Self, PubKeyError> {
        let bytes = hex::decode(s).map_err(|e| PubKeyError::Hex(e.to_string()))?;
        let len = bytes.len();
        let arr: [u8; 33] = bytes
            .try_into()
            .map_err(|_| PubKeyError::WrongLength(len))?;
        Ok(Self(arr))
    }

    /// Hex-encoded representation.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Raw 33-byte slice.
    pub fn as_bytes(&self) -> &[u8; 33] {
        &self.0
    }
}

/// A change to a multisig authority's signer set and/or threshold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultisigUpdate {
    /// Which authority is being updated. Protocol rule: must equal the `Authority` of
    /// the `Proposal` that carries this action (a role can only modify its own config).
    pub role: Authority,
    pub add_keys: Vec<CompressedPubKey>,
    pub remove_keys: Vec<CompressedPubKey>,
    pub new_threshold: NonZeroU8,
}

/// A verification key update action.
///
/// The `authority` determines the wire action variant:
/// - `StrataAdmin` → `OlStfVkUpdate` (OL STF predicate in CheckpointState)
/// - `AlpenAdmin`  → `EeStfVkUpdate` (EE predicate)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VkUpdate {
    pub authority: Authority,
    /// `PredicateTypeId` as raw byte: 0=NeverAccept, 1=AlwaysAccept, 10=Bip340Schnorr, 20=Sp1Groth16
    pub type_id: u8,
    /// Backend-specific condition bytes (empty for Never/Always, 32 for Schnorr, 356 for SP1).
    pub condition: Vec<u8>,
}

/// A governance action that a signer can propose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    MultisigUpdate(MultisigUpdate),
    VkUpdate(VkUpdate),
    OperatorSetUpdate(OperatorSetUpdate),
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_HEX: &str = "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";
    // secp256k1 generator G x-coordinate (32 bytes, even parity)
    const EVEN_KEY_HEX: &str = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

    #[test]
    fn test_compressed_pubkey_from_hex_ok() {
        let pk = CompressedPubKey::from_hex(VALID_HEX).expect("valid 33-byte hex");
        assert_eq!(pk.to_hex(), VALID_HEX);
    }

    #[test]
    fn test_compressed_pubkey_rejects_short_length() {
        let short = "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709e";
        let err = CompressedPubKey::from_hex(short).unwrap_err();
        assert!(matches!(err, PubKeyError::WrongLength(32)));
    }

    #[test]
    fn test_compressed_pubkey_rejects_invalid_hex() {
        let err = CompressedPubKey::from_hex("zz").unwrap_err();
        assert!(matches!(err, PubKeyError::Hex(_)));
    }

    #[test]
    fn test_action_builds() {
        let pk = CompressedPubKey::from_hex(VALID_HEX).unwrap();
        let update = MultisigUpdate {
            role: Authority::StrataAdmin,
            add_keys: vec![pk],
            remove_keys: vec![],
            new_threshold: NonZeroU8::new(2).unwrap(),
        };
        let action = Action::MultisigUpdate(update.clone());
        match action {
            Action::MultisigUpdate(u) => assert_eq!(u, update),
            Action::VkUpdate(_) | Action::OperatorSetUpdate(_) => panic!("unexpected variant"),
        }
    }

    #[test]
    fn test_even_pubkey_from_hex_ok() {
        let pk = EvenPubKey::from_hex(EVEN_KEY_HEX).expect("valid 32-byte x-only hex");
        assert_eq!(pk.to_hex(), EVEN_KEY_HEX);
    }

    #[test]
    fn test_even_pubkey_rejects_33_byte_hex() {
        // 33-byte compressed key (66 hex chars) — must be rejected
        let err = EvenPubKey::from_hex(VALID_HEX).unwrap_err();
        assert!(matches!(err, EvenPubKeyError::WrongLength(33)));
    }

    #[test]
    fn test_even_pubkey_rejects_invalid_hex() {
        let err = EvenPubKey::from_hex("zz").unwrap_err();
        assert!(matches!(err, EvenPubKeyError::Hex(_)));
    }

    #[test]
    fn test_operator_set_update_builds() {
        let pk = EvenPubKey::from_hex(EVEN_KEY_HEX).unwrap();
        let update = OperatorSetUpdate {
            add_members: vec![pk],
            remove_members: vec![5],
        };
        assert_eq!(update.add_members.len(), 1);
        assert_eq!(update.remove_members, vec![5u32]);
    }
}
