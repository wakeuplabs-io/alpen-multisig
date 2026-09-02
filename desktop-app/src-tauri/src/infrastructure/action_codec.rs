//! Codec between the client domain `Action` and the Strata-owned `MultisigAction`
//! SSZ form.
//!
//! This is the **only** module that imports `strata_asm_*` / `strata_crypto` crates.
//! Everything else in the desktop application talks in domain types.

use std::num::NonZeroU8;

use ssz::{Decode, Encode};
use strata_asm_txs_admin::actions::updates::{
    AlpenAdminMultisigUpdate, Defcon1Update, Defcon3Update, EeStfVkUpdate, OlStfVkUpdate,
    OperatorSetUpdate as StrataOperatorSetUpdate, SequencerUpdate as StrataSequencerUpdate,
    StrataAdminMultisigUpdate, StrataSeqManagerMultisigUpdate,
};
use strata_asm_txs_admin::actions::{CancelAction, MultisigAction, UpdateAction};
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::ThresholdConfigUpdate;
use strata_crypto::EvenPublicKey;
use strata_identifiers::Buf32;
use strata_predicate::{PredicateKey, PredicateTypeId};

use crate::domain::action::{
    Action, CompressedPubKey, EvenPubKey, MultisigUpdate, OperatorSetUpdate, PubKeyError,
    SequencerKeyUpdate, VkUpdate,
};
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
    #[error("unsupported authority: {0}")]
    UnsupportedAuthority(String),
    #[error("unsupported predicate type id: {0}")]
    UnsupportedPredicateType(u8),
    #[error("invalid predicate condition: {0}")]
    InvalidCondition(String),
}

/// Encodes a domain `Action` to canonical SSZ bytes (the signed form).
pub fn encode(action: &Action) -> Result<Vec<u8>, CodecError> {
    let strata = to_strata_action(action)?;
    Ok(strata.as_ssz_bytes())
}

/// Decodes canonical SSZ bytes into a domain `Action`.
pub fn decode(bytes: &[u8]) -> Result<Action, CodecError> {
    let strata =
        MultisigAction::from_ssz_bytes(bytes).map_err(|e| CodecError::Decode(format!("{e:?}")))?;
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

/// Wraps an existing update action hex in a `MultisigAction::Cancel` envelope.
///
/// `target_action_hex` must encode a `MultisigAction::Update`. `target_seq_no` is the
/// seq_no of the queued update (used as the cancel's `target_id`).
pub fn encode_cancel_hex_for_target(
    target_action_hex: &str,
    target_seq_no: u64,
) -> Result<String, CodecError> {
    let hex = target_action_hex
        .strip_prefix("0x")
        .unwrap_or(target_action_hex);
    let bytes = hex::decode(hex).map_err(|e| CodecError::Hex(e.to_string()))?;
    let target_action =
        MultisigAction::from_ssz_bytes(&bytes).map_err(|e| CodecError::Decode(format!("{e:?}")))?;
    let update = match target_action {
        MultisigAction::Update(u) => u,
        MultisigAction::Cancel(_) => return Err(CodecError::UnsupportedVariant("Cancel")),
    };
    let target_id: u32 = target_seq_no
        .try_into()
        .map_err(|_| CodecError::Encode(format!("seq_no {target_seq_no} exceeds u32 range")))?;
    let cancel = MultisigAction::Cancel(CancelAction::new(target_id, update));
    Ok(hex::encode(cancel.as_ssz_bytes()))
}

// ─── Domain → Strata ────────────────────────────────────────────────────────

fn to_strata_action(action: &Action) -> Result<MultisigAction, CodecError> {
    match action {
        Action::MultisigUpdate(update) => {
            let config_update = threshold_config_update_from_domain(update)?;
            match update.role {
                Authority::StrataAdmin => {
                    Ok(MultisigAction::Update(UpdateAction::StrataAdminMultisig(
                        StrataAdminMultisigUpdate::new(config_update),
                    )))
                }
                Authority::SequencerManager => Ok(MultisigAction::Update(
                    UpdateAction::StrataSeqManagerMultisig(StrataSeqManagerMultisigUpdate::new(
                        config_update,
                    )),
                )),
                Authority::AlpenAdmin => Ok(MultisigAction::Update(
                    UpdateAction::AlpenAdminMultisig(AlpenAdminMultisigUpdate::new(config_update)),
                )),
                other => Err(CodecError::UnsupportedAuthority(format!(
                    "encoding not implemented for authority `{other:?}`"
                ))),
            }
        }
        Action::VkUpdate(update) => {
            let predicate = predicate_key_from_domain(update)?;
            match update.authority {
                Authority::StrataAdmin => Ok(MultisigAction::Update(UpdateAction::OlStfVk(
                    OlStfVkUpdate::new(predicate),
                ))),
                Authority::AlpenAdmin => Ok(MultisigAction::Update(UpdateAction::EeStfVk(
                    EeStfVkUpdate::new(predicate),
                ))),
                other => Err(CodecError::UnsupportedAuthority(format!(
                    "vk update not implemented for authority `{other:?}`"
                ))),
            }
        }
        Action::OperatorSetUpdate(update) => {
            let add = update
                .add_members
                .iter()
                .map(to_strata_even_pubkey)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(MultisigAction::Update(UpdateAction::OperatorSet(
                StrataOperatorSetUpdate::new(add, update.remove_members.clone()),
            )))
        }
        Action::SequencerKeyUpdate(update) => Ok(MultisigAction::Update(UpdateAction::Sequencer(
            StrataSequencerUpdate::new(Buf32(*update.new_pub_key.as_bytes())),
        ))),
        Action::Defcon1 => Ok(MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update))),
        Action::Defcon3 => Ok(MultisigAction::Update(UpdateAction::Defcon3(Defcon3Update))),
    }
}

fn predicate_key_from_domain(update: &VkUpdate) -> Result<PredicateKey, CodecError> {
    let type_id = PredicateTypeId::try_from(update.type_id)
        .map_err(|_| CodecError::UnsupportedPredicateType(update.type_id))?;
    Ok(PredicateKey::new(type_id, update.condition.clone()))
}

fn threshold_config_update_from_domain(
    update: &MultisigUpdate,
) -> Result<ThresholdConfigUpdate, CodecError> {
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
    Ok(ThresholdConfigUpdate::new(add_keys, remove_keys, threshold))
}

fn to_strata_pubkey(pk: &CompressedPubKey) -> Result<CompressedPublicKey, CodecError> {
    CompressedPublicKey::from_slice(pk.as_bytes())
        .map_err(|e| CodecError::Encode(format!("invalid pubkey: {e}")))
}

// ─── Strata → Domain ────────────────────────────────────────────────────────

fn from_strata_action(action: MultisigAction) -> Result<Action, CodecError> {
    match action {
        MultisigAction::Update(UpdateAction::StrataAdminMultisig(update)) => {
            let domain_update =
                multisig_update_from_threshold_config(Authority::StrataAdmin, update.config())?;
            Ok(Action::MultisigUpdate(domain_update))
        }
        MultisigAction::Update(UpdateAction::StrataSeqManagerMultisig(update)) => {
            let domain_update = multisig_update_from_threshold_config(
                Authority::SequencerManager,
                update.config(),
            )?;
            Ok(Action::MultisigUpdate(domain_update))
        }
        MultisigAction::Update(UpdateAction::AlpenAdminMultisig(update)) => {
            let domain_update =
                multisig_update_from_threshold_config(Authority::AlpenAdmin, update.config())?;
            Ok(Action::MultisigUpdate(domain_update))
        }
        MultisigAction::Update(UpdateAction::OperatorSet(u)) => {
            let (add_strata, remove) = u.into_inner();
            let add = add_strata.iter().map(from_strata_even_pubkey).collect();
            Ok(Action::OperatorSetUpdate(OperatorSetUpdate {
                add_members: add,
                remove_members: remove,
            }))
        }
        MultisigAction::Update(UpdateAction::Sequencer(update)) => {
            let new_pub_key = EvenPubKey::new(update.into_inner().0);
            Ok(Action::SequencerKeyUpdate(SequencerKeyUpdate {
                new_pub_key,
            }))
        }
        MultisigAction::Update(UpdateAction::OlStfVk(update)) => {
            let key = update.into_key();
            Ok(Action::VkUpdate(VkUpdate {
                authority: Authority::StrataAdmin,
                type_id: key.id(),
                condition: key.condition().to_vec(),
            }))
        }
        MultisigAction::Update(UpdateAction::EeStfVk(update)) => {
            let key = update.into_key();
            Ok(Action::VkUpdate(VkUpdate {
                authority: Authority::AlpenAdmin,
                type_id: key.id(),
                condition: key.condition().to_vec(),
            }))
        }
        MultisigAction::Update(UpdateAction::AsmStfVk(_)) => {
            Err(CodecError::UnsupportedVariant("AsmStfVk"))
        }
        // Security Council actions — decoded explicitly so a future upstream variant fails to
        // compile here instead of silently falling through. Each becomes a real arm as its
        // slice lands; see docs/specs/security-council.md.
        MultisigAction::Update(UpdateAction::StrataSecurityCouncilMultisig(_)) => Err(
            CodecError::UnsupportedVariant("StrataSecurityCouncilMultisig"),
        ),
        MultisigAction::Update(UpdateAction::Defcon1(_)) => Ok(Action::Defcon1),
        MultisigAction::Update(UpdateAction::Defcon3(_)) => Ok(Action::Defcon3),
        MultisigAction::Update(UpdateAction::SafeHarbourAddress(_)) => {
            Err(CodecError::UnsupportedVariant("SafeHarbourAddress"))
        }
        MultisigAction::Cancel(_) => Err(CodecError::UnsupportedVariant("Cancel")),
    }
}

fn multisig_update_from_threshold_config(
    role: Authority,
    config: &ThresholdConfigUpdate,
) -> Result<MultisigUpdate, CodecError> {
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

fn to_strata_even_pubkey(pk: &EvenPubKey) -> Result<EvenPublicKey, CodecError> {
    // EvenPublicKey::from(XOnlyPublicKey) normalises parity to even — safe for x-only keys.
    use bitcoin::secp256k1::XOnlyPublicKey;
    let x_only = XOnlyPublicKey::from_slice(pk.as_bytes())
        .map_err(|e| CodecError::Encode(format!("invalid x-only pubkey: {e}")))?;
    Ok(EvenPublicKey::from(x_only))
}

fn from_strata_even_pubkey(pk: &EvenPublicKey) -> EvenPubKey {
    // EvenPublicKey derefs to secp256k1 PublicKey; x_only_public_key().0 gives XOnlyPublicKey.
    EvenPubKey::new(pk.x_only_public_key().0.serialize())
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

    /// The round trip alone would pass on a codec that agreed with itself and with nobody else —
    /// mapping both arms to Defcon *3* round-trips just as happily. The tx type is what pins the
    /// bytes to the action the Security Council means to sign.
    #[test]
    fn defcon_1_round_trips_and_encodes_upstreams_defcon_1_tx_type() {
        use strata_asm_params::UpdateTxType;

        let encoded = encode(&Action::Defcon1).expect("encode ok");

        assert_eq!(decode(&encoded).expect("decode ok"), Action::Defcon1);

        let upstream = MultisigAction::from_ssz_bytes(&encoded).expect("upstream decodes it");
        let MultisigAction::Update(update) = upstream else {
            panic!("Defcon 1 is an update, not a cancel");
        };
        assert_eq!(update.update_tx_type(), UpdateTxType::Defcon1);
    }

    /// Both Defcon payloads are empty unit structs, so the only thing separating their bytes is
    /// the SSZ union selector. A codec with the two encode arms crossed would round-trip just as
    /// happily and hand the council the other lever to sign — which is what the last assertion,
    /// and not the round trip, is here to catch.
    #[test]
    fn defcon_3_round_trips_and_encodes_upstreams_defcon_3_tx_type() {
        use strata_asm_params::UpdateTxType;

        let encoded = encode(&Action::Defcon3).expect("encode ok");

        assert_eq!(decode(&encoded).expect("decode ok"), Action::Defcon3);

        let upstream = MultisigAction::from_ssz_bytes(&encoded).expect("upstream decodes it");
        let MultisigAction::Update(update) = upstream else {
            panic!("Defcon 3 is an update, not a cancel");
        };
        assert_eq!(update.update_tx_type(), UpdateTxType::Defcon3);

        assert_ne!(
            encode(&Action::Defcon1).expect("encode ok"),
            encoded,
            "the immediate and the timelocked lever must not encode to the same bytes"
        );
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
        let strata_update = StrataAdminMultisigUpdate::new(config_update);
        let strata_action =
            MultisigAction::Update(UpdateAction::StrataAdminMultisig(strata_update));
        let direct_bytes = strata_action.as_ssz_bytes();

        let domain_bytes = encode(&sample_action()).unwrap();
        assert_eq!(domain_bytes, direct_bytes);
    }

    fn sample_alpen_admin_action() -> Action {
        let pk = CompressedPubKey::from_hex(VALID_HEX).unwrap();
        Action::MultisigUpdate(MultisigUpdate {
            role: Authority::AlpenAdmin,
            add_keys: vec![pk],
            remove_keys: vec![],
            new_threshold: NonZeroU8::new(2).unwrap(),
        })
    }

    #[test]
    fn test_alpen_admin_multisig_roundtrip_hex() {
        let action = sample_alpen_admin_action();
        let encoded = encode_hex(&action).expect("encode ok");
        let decoded = decode_hex(&encoded).expect("decode ok");
        assert_eq!(decoded, action);
    }

    #[test]
    fn test_alpen_admin_multisig_roundtrip_bytes() {
        let action = sample_alpen_admin_action();
        let bytes = encode(&action).expect("encode ok");
        let decoded = decode(&bytes).expect("decode ok");
        assert_eq!(decoded, action);
    }

    #[test]
    fn test_alpen_admin_multisig_encode_matches_direct_strata_ssz() {
        let pk_bytes = hex::decode(VALID_HEX).unwrap();
        let secp_pk = bitcoin::secp256k1::PublicKey::from_slice(&pk_bytes).unwrap();
        let strata_pk = CompressedPublicKey::from(secp_pk);
        let config_update =
            ThresholdConfigUpdate::new(vec![strata_pk], vec![], std::num::NonZero::new(2).unwrap());
        let strata_update = AlpenAdminMultisigUpdate::new(config_update);
        let strata_action = MultisigAction::Update(UpdateAction::AlpenAdminMultisig(strata_update));
        let direct_bytes = strata_action.as_ssz_bytes();

        let domain_bytes = encode(&sample_alpen_admin_action()).unwrap();
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

    fn sample_operator_set_action() -> Action {
        // secp256k1 generator G x-coordinate — a canonical, even-parity x-only key.
        let even_key_hex = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let pk = EvenPubKey::from_hex(even_key_hex).unwrap();
        Action::OperatorSetUpdate(OperatorSetUpdate {
            add_members: vec![pk],
            remove_members: vec![5],
        })
    }

    #[test]
    fn test_operator_set_roundtrip_bytes() {
        let action = sample_operator_set_action();
        let bytes = encode(&action).expect("encode ok");
        let decoded = decode(&bytes).expect("decode ok");
        assert_eq!(decoded, action);
    }

    #[test]
    fn test_operator_set_roundtrip_hex() {
        let action = sample_operator_set_action();
        let hex = encode_hex(&action).expect("encode ok");
        let decoded = decode_hex(&hex).expect("decode ok");
        assert_eq!(decoded, action);
    }

    fn sample_seq_manager_action() -> Action {
        let pk = CompressedPubKey::from_hex(VALID_HEX).unwrap();
        Action::MultisigUpdate(MultisigUpdate {
            role: Authority::SequencerManager,
            add_keys: vec![pk],
            remove_keys: vec![],
            new_threshold: NonZeroU8::new(2).unwrap(),
        })
    }

    #[test]
    fn test_seq_manager_multisig_roundtrip_hex() {
        let action = sample_seq_manager_action();
        let encoded = encode_hex(&action).expect("encode ok");
        let decoded = decode_hex(&encoded).expect("decode ok");
        assert_eq!(decoded, action);
    }

    #[test]
    fn test_seq_manager_multisig_roundtrip_bytes() {
        let action = sample_seq_manager_action();
        let bytes = encode(&action).expect("encode ok");
        let decoded = decode(&bytes).expect("decode ok");
        assert_eq!(decoded, action);
    }

    #[test]
    fn test_seq_manager_multisig_encode_matches_direct_strata_ssz() {
        let pk_bytes = hex::decode(VALID_HEX).unwrap();
        let secp_pk = bitcoin::secp256k1::PublicKey::from_slice(&pk_bytes).unwrap();
        let strata_pk = CompressedPublicKey::from(secp_pk);
        let config_update =
            ThresholdConfigUpdate::new(vec![strata_pk], vec![], std::num::NonZero::new(2).unwrap());
        let strata_update = StrataSeqManagerMultisigUpdate::new(config_update);
        let strata_action =
            MultisigAction::Update(UpdateAction::StrataSeqManagerMultisig(strata_update));
        let direct_bytes = strata_action.as_ssz_bytes();

        let domain_bytes = encode(&sample_seq_manager_action()).unwrap();
        assert_eq!(domain_bytes, direct_bytes);
    }

    fn sample_sequencer_key_update_action() -> Action {
        let even_key_hex = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let pk = EvenPubKey::from_hex(even_key_hex).unwrap();
        Action::SequencerKeyUpdate(SequencerKeyUpdate { new_pub_key: pk })
    }

    #[test]
    fn test_sequencer_key_update_roundtrip_hex() {
        let action = sample_sequencer_key_update_action();
        let encoded = encode_hex(&action).expect("encode ok");
        let decoded = decode_hex(&encoded).expect("decode ok");
        assert_eq!(decoded, action);
    }

    #[test]
    fn test_sequencer_key_update_roundtrip_bytes() {
        let action = sample_sequencer_key_update_action();
        let bytes = encode(&action).expect("encode ok");
        let decoded = decode(&bytes).expect("decode ok");
        assert_eq!(decoded, action);
    }

    #[test]
    fn test_sequencer_key_update_encode_matches_direct_strata_ssz() {
        let even_key_hex = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let bytes = hex::decode(even_key_hex).unwrap();
        let strata_update = StrataSequencerUpdate::new(Buf32(bytes.try_into().unwrap()));
        let strata_action = MultisigAction::Update(UpdateAction::Sequencer(strata_update));
        let direct_bytes = strata_action.as_ssz_bytes();

        let domain_bytes = encode(&sample_sequencer_key_update_action()).unwrap();
        assert_eq!(domain_bytes, direct_bytes);
    }

    #[test]
    fn test_operator_set_encode_matches_direct_strata_ssz() {
        use bitcoin::secp256k1::XOnlyPublicKey;
        use strata_asm_txs_admin::actions::updates::OperatorSetUpdate as StrataOsu;
        use strata_crypto::EvenPublicKey;

        let even_key_hex = "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
        let bytes = hex::decode(even_key_hex).unwrap();
        let x_only = XOnlyPublicKey::from_slice(&bytes).unwrap();
        let strata_pk = EvenPublicKey::from(x_only);

        let strata_update = StrataOsu::new(vec![strata_pk], vec![5]);
        let strata_action = MultisigAction::Update(UpdateAction::OperatorSet(strata_update));
        let direct_bytes = strata_action.as_ssz_bytes();

        let domain_bytes = encode(&sample_operator_set_action()).unwrap();
        assert_eq!(domain_bytes, direct_bytes);
    }
}
