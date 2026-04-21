//! Governance actions that a signer can propose — client-side domain.
//!
//! These types mirror (a subset of) the protocol's `MultisigAction` without depending
//! on Strata crates. Translation to/from the canonical SSZ form lives in
//! `crate::infrastructure::action_codec`.

use std::num::NonZeroU8;

use crate::domain::authority::Authority;

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

/// A governance action that a signer can propose.
///
/// Single variant for POC-4; more variants (`Cancel`, `OperatorSetUpdate`, etc.) will
/// be added as the feature set grows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    MultisigUpdate(MultisigUpdate),
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_HEX: &str = "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";

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
        }
    }
}
