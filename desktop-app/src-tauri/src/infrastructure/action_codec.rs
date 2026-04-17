//! Codec between the client domain `Action` and the Strata-owned `MultisigAction`
//! SSZ form.
//!
//! This is the **only** module that imports `strata_asm_*` / `strata_crypto` crates.
//! Everything else in the desktop application talks in domain types.

use std::num::NonZeroU8;

use ssz::{Decode, Encode};
use strata_asm_params::Role;
use strata_asm_txs_admin::actions::updates::multisig::MultisigUpdate as StrataMultisigUpdate;
use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::ThresholdConfigUpdate;

use crate::domain::action::{Action, CompressedPubKey, MultisigUpdate, PubKeyError};
use crate::domain::authority::Authority;

/// Errors produced when encoding/decoding an `Action`.
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("ssz serialization failed: {0}")]
    Encode(String),
    #[error("ssz deserialization failed: {0}")]
    Decode(String),
    #[error("invalid hex: {0}")]
    Hex(String),
    #[error("unsupported action variant: {0}")]
    UnsupportedVariant(&'static str),
    #[error("invalid public key: {0}")]
    PubKey(#[from] PubKeyError),
    #[error("invalid threshold: value must be non-zero")]
    InvalidThreshold,
}

/// Encodes a domain `Action` to canonical SSZ bytes (the signed form).
pub fn encode(action: &Action) -> Result<Vec<u8>, CodecError> {
    let strata = to_strata_action(action)?;
    Ok(strata.as_ssz_bytes())
}

/// Decodes canonical SSZ bytes into a domain `Action`.
pub fn decode(bytes: &[u8]) -> Result<Action, CodecError> {
    let strata = MultisigAction::from_ssz_bytes(bytes)
        .map_err(|e| CodecError::Decode(format!("{e:?}")))?;
    from_strata_action(strata)
}

/// Encodes a domain `Action` to a hex string (what the orchestrator stores in
/// `action_hex`).
pub fn encode_hex(action: &Action) -> Result<String, CodecError> {
    Ok(hex::encode(encode(action)?))
}

/// Decodes a hex `action_hex` into a domain `Action`.
pub fn decode_hex(s: &str) -> Result<Action, CodecError> {
    let bytes = hex::decode(s).map_err(|e| CodecError::Hex(e.to_string()))?;
    decode(&bytes)
}

// ─── Domain → Strata ────────────────────────────────────────────────────────

fn to_strata_action(action: &Action) -> Result<MultisigAction, CodecError> {
    match action {
        Action::MultisigUpdate(update) => {
            let strata_update = to_strata_multisig_update(update)?;
            Ok(MultisigAction::Update(UpdateAction::Multisig(
                strata_update,
            )))
        }
    }
}

fn to_strata_multisig_update(update: &MultisigUpdate) -> Result<StrataMultisigUpdate, CodecError> {
    let add_keys = update
        .add_keys
        .iter()
        .map(to_strata_pubkey)
        .collect::<Result<Vec<_>, _>>()?;
    let remove_keys = update
        .remove_keys
        .iter()
        .map(to_strata_pubkey)
        .collect::<Result<Vec<_>, _>>()?;
    let threshold =
        std::num::NonZero::new(update.new_threshold.get()).ok_or(CodecError::InvalidThreshold)?;
    let config_update = ThresholdConfigUpdate::new(add_keys, remove_keys, threshold);
    Ok(StrataMultisigUpdate::new(
        config_update,
        to_strata_role(update.role),
    ))
}

fn to_strata_pubkey(pk: &CompressedPubKey) -> Result<CompressedPublicKey, CodecError> {
    CompressedPublicKey::from_slice(pk.as_bytes())
        .map_err(|e| CodecError::Encode(format!("invalid pubkey: {e}")))
}

fn to_strata_role(authority: Authority) -> Role {
    match authority {
        Authority::StrataAdmin => Role::StrataAdministrator,
    }
}

// ─── Strata → Domain ────────────────────────────────────────────────────────

fn from_strata_action(action: MultisigAction) -> Result<Action, CodecError> {
    match action {
        MultisigAction::Update(UpdateAction::Multisig(update)) => {
            let domain_update = from_strata_multisig_update(update)?;
            Ok(Action::MultisigUpdate(domain_update))
        }
        MultisigAction::Update(UpdateAction::OperatorSet(_)) => {
            Err(CodecError::UnsupportedVariant("OperatorSet"))
        }
        MultisigAction::Update(UpdateAction::Sequencer(_)) => {
            Err(CodecError::UnsupportedVariant("Sequencer"))
        }
        MultisigAction::Update(UpdateAction::VerifyingKey(_)) => {
            Err(CodecError::UnsupportedVariant("VerifyingKey"))
        }
        MultisigAction::Cancel(_) => Err(CodecError::UnsupportedVariant("Cancel")),
    }
}

fn from_strata_multisig_update(update: StrataMultisigUpdate) -> Result<MultisigUpdate, CodecError> {
    let role = from_strata_role(update.role())?;
    let config = update.config();
    let add_keys = config
        .add_members()
        .iter()
        .map(from_strata_pubkey)
        .collect::<Result<Vec<_>, _>>()?;
    let remove_keys = config
        .remove_members()
        .iter()
        .map(from_strata_pubkey)
        .collect::<Result<Vec<_>, _>>()?;
    let new_threshold =
        NonZeroU8::new(config.new_threshold().get()).ok_or(CodecError::InvalidThreshold)?;
    Ok(MultisigUpdate {
        role,
        add_keys,
        remove_keys,
        new_threshold,
    })
}

fn from_strata_pubkey(pk: &CompressedPublicKey) -> Result<CompressedPubKey, CodecError> {
    Ok(CompressedPubKey::new(pk.serialize()))
}

fn from_strata_role(role: Role) -> Result<Authority, CodecError> {
    match role {
        Role::StrataAdministrator => Ok(Authority::StrataAdmin),
        _ => Err(CodecError::UnsupportedVariant("non-StrataAdmin role")),
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_HEX: &str = "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";

    fn sample_action() -> Action {
        let pk = CompressedPubKey::from_hex(VALID_HEX).unwrap();
        Action::MultisigUpdate(MultisigUpdate {
            role: Authority::StrataAdmin,
            add_keys: vec![pk],
            remove_keys: vec![],
            new_threshold: NonZeroU8::new(2).unwrap(),
        })
    }

    #[test]
    fn test_roundtrip_hex() {
        let action = sample_action();
        let encoded = encode_hex(&action).expect("encode ok");
        let decoded = decode_hex(&encoded).expect("decode ok");
        assert_eq!(decoded, action);
    }

    #[test]
    fn test_roundtrip_bytes() {
        let action = sample_action();
        let bytes = encode(&action).expect("encode ok");
        let decoded = decode(&bytes).expect("decode ok");
        assert_eq!(decoded, action);
    }

    #[test]
    fn test_encode_matches_direct_strata_ssz() {
        // Guarantee the encoding stays byte-compatible with the direct Strata call
        // — what the upstream crate produces for the same `MultisigAction`.
        let pk_bytes = hex::decode(VALID_HEX).unwrap();
        let secp_pk = bitcoin::secp256k1::PublicKey::from_slice(&pk_bytes).unwrap();
        let strata_pk = CompressedPublicKey::from(secp_pk);
        let config_update =
            ThresholdConfigUpdate::new(vec![strata_pk], vec![], std::num::NonZero::new(2).unwrap());
        let strata_update = StrataMultisigUpdate::new(config_update, Role::StrataAdministrator);
        let strata_action = MultisigAction::Update(UpdateAction::Multisig(strata_update));
        let direct_bytes = strata_action.as_ssz_bytes();

        let domain_bytes = encode(&sample_action()).unwrap();
        assert_eq!(domain_bytes, direct_bytes);
    }

    #[test]
    fn test_decode_rejects_malformed_hex() {
        let err = decode_hex("zz").unwrap_err();
        assert!(matches!(err, CodecError::Hex(_)));
    }

    #[test]
    fn test_decode_rejects_truncated_bytes() {
        let err = decode(&[0x00, 0x01, 0x02]).unwrap_err();
        assert!(matches!(err, CodecError::Decode(_)));
    }
}
