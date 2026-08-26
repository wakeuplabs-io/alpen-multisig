//! Coordination hygiene: detect when ASM canonical state reflects a proposal's governance change.
//!
//! This does not re-validate signatures or queue semantics. Concurrent overlapping updates may
//! produce ambiguous post-condition matches (same class of risk as threshold snapshot checks).

use serde_json::{json, Value};
use ssz::Decode;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_params::Role;
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};
use strata_asm_proto_bridge_v1::{BridgeV1State, BridgeV1Subproto};
use strata_asm_proto_checkpoint::CheckpointState;
use strata_asm_proto_checkpoint::CheckpointSubprotocol;
use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
use strata_crypto::threshold_signature::ThresholdConfigUpdate;
use strata_predicate::{PredicateKey, PredicateTypeId};

use crate::domain::authority::Authority;
use crate::error::AppError;
use crate::infrastructure::{action_codec, rpc_timeout};

#[cfg(any(test, feature = "dev-mocks"))]
const MOCK_MEMBERSHIP_URL: &str = "mock://asm-membership";
#[cfg(any(test, feature = "dev-mocks"))]
const MOCK_ENACTED_URL: &str = "mock://asm-enacted";

/// Returns true when live ASM canonical state satisfies the post-conditions of `action_hex`.
pub(crate) async fn is_proposal_enacted_on_asm(
    rpc_url: &str,
    authority: Authority,
    seq_no: u64,
    action_hex: &str,
) -> Result<bool, AppError> {
    if let Some(enacted) = mock_is_enacted(rpc_url) {
        return Ok(enacted);
    }

    let action =
        action_codec::decode_multisig_action_hex(action_hex).map_err(AppError::BadRequest)?;

    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([]))
        .await
        .map_err(AppError::BadRequest)?;
    let anchor = decode_anchor_state_from_status(&status_result).map_err(AppError::BadRequest)?;

    match &action {
        MultisigAction::Update(UpdateAction::OlStfVk(update)) => {
            let checkpoint = decode_checkpoint_state(&anchor).map_err(AppError::BadRequest)?;
            Ok(predicate_keys_match(
                update.key(),
                checkpoint.checkpoint_predicate(),
            ))
        }
        MultisigAction::Update(UpdateAction::EeStfVk(update)) => {
            if authority != Authority::AlpenAdmin {
                return Err(AppError::BadRequest(
                    "EeStfVk proposal requires AlpenAdmin authority".to_string(),
                ));
            }
            let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;
            let alpen = admin.authority(Role::AlpenAdministrator).ok_or_else(|| {
                AppError::BadRequest(
                    "admin state missing authority for role `AlpenAdministrator`".to_string(),
                )
            })?;
            let target = UpdateAction::EeStfVk(update.clone());
            let still_queued = admin.queued().iter().any(|q| q.action() == &target);
            Ok(ee_stf_vk_enacted(alpen.last_seqno(), seq_no, still_queued))
        }
        MultisigAction::Update(UpdateAction::Sequencer(update)) => {
            // SequencerUpdate carries a Buf32 (raw 32-byte key). The ASM handler wraps it as
            // PredicateKey::new(Bip340Schnorr, key_bytes) and relays it to the checkpoint
            // subprotocol, which stores it in `sequencer_predicate` (distinct from
            // `checkpoint_predicate` which OlStfVk updates).
            let checkpoint = decode_checkpoint_state(&anchor).map_err(AppError::BadRequest)?;
            let expected =
                PredicateKey::new(PredicateTypeId::Bip340Schnorr, update.pub_key().0.to_vec());
            Ok(predicate_keys_match(
                &expected,
                checkpoint.sequencer_predicate(),
            ))
        }
        MultisigAction::Update(UpdateAction::OperatorSet(update)) => {
            let bridge = decode_bridge_state(&anchor).map_err(AppError::BadRequest)?;
            let current_keys: Vec<String> = bridge
                .operators()
                .operators()
                .iter()
                .map(|e| hex::encode(e.musig2_pk().x_only_public_key().0.serialize()))
                .collect();
            let (add_members, remove_members) = update.clone().into_inner();
            Ok(operator_set_post_conditions_met(
                &current_keys,
                &add_members,
                &remove_members,
            ))
        }
        // Security Council actions. Explicit arms rather than a catch-all: without them these
        // would fall through to the multisig-config branch, which returns `Ok(false)` for an
        // unrecognized variant — a Defcon proposal would silently never reach `Enacted`. Defcon 1
        // has its post-condition (V1); the remaining two gain theirs as their slice lands. See
        // docs/specs/security-council.md and docs/specs/security-council-defcon-phase-4.md.
        MultisigAction::Update(UpdateAction::Defcon1(_)) => {
            let bridge = decode_bridge_state(&anchor).map_err(AppError::BadRequest)?;
            let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;
            let safe_harbour_activated = bridge.safe_harbour().is_activated();
            let defcon1_queued = admin
                .queued()
                .iter()
                .any(|q| matches!(q.action(), UpdateAction::Defcon1(_)));
            Ok(defcon1_enacted(safe_harbour_activated, defcon1_queued))
        }
        MultisigAction::Update(UpdateAction::Defcon3(_)) => Err(AppError::BadRequest(
            "Defcon3 enactment detection is not implemented yet".to_string(),
        )),
        MultisigAction::Update(UpdateAction::SafeHarbourAddress(_)) => Err(AppError::BadRequest(
            "SafeHarbourAddress enactment detection is not implemented yet".to_string(),
        )),
        MultisigAction::Update(
            UpdateAction::StrataAdminMultisig(_)
            | UpdateAction::StrataSeqManagerMultisig(_)
            | UpdateAction::AlpenAdminMultisig(_)
            | UpdateAction::StrataSecurityCouncilMultisig(_)
            | UpdateAction::AsmStfVk(_),
        ) => {
            let Some(config_update) = extract_multisig_config_update(&action, authority)? else {
                return Ok(false);
            };
            let role = authority_to_role(authority).map_err(AppError::BadRequest)?;
            let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;
            let authority_config = admin.authority(role).ok_or_else(|| {
                AppError::BadRequest(format!(
                    "admin state missing authority for role `{:?}`",
                    role
                ))
            })?;

            let canonical_keys: Vec<String> = authority_config
                .config()
                .keys()
                .iter()
                .map(|k| hex::encode(k.serialize()))
                .collect();
            let threshold = authority_config.config().threshold();
            let last_seqno = authority_config.last_seqno();

            Ok(multisig_update_post_conditions_met(
                &canonical_keys,
                threshold,
                last_seqno,
                seq_no,
                config_update,
            ))
        }
        MultisigAction::Cancel(cancel) => {
            let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;
            Ok(admin.find_queued(cancel.target_id()).is_none())
        }
    }
}

fn predicate_keys_match(proposed: &PredicateKey, current: &PredicateKey) -> bool {
    proposed.id() == current.id() && proposed.condition() == current.condition()
}

/// EE STF VK updates emit an `EePredicateKeyUpdate` manifest log (no checkpoint field).
/// Treat as enacted once the reveal consumed the seqno and the update left the admin queue.
fn ee_stf_vk_enacted(last_seqno: u64, seq_no: u64, still_queued: bool) -> bool {
    last_seqno >= seq_no && !still_queued
}

/// Defcon 1 executes at depth 0: it activates the safe harbour in the reveal block and never
/// enters the admin queue. A queued Defcon 1 means upstream changed that depth, not that this
/// proposal enacted.
fn defcon1_enacted(safe_harbour_activated: bool, defcon1_queued: bool) -> bool {
    safe_harbour_activated && !defcon1_queued
}

/// Returns `Some(config)` for known multisig-update authority/variant pairs, `None` for
/// non-multisig-update action variants, and an error for genuine authority/variant mismatches.
fn extract_multisig_config_update(
    action: &MultisigAction,
    authority: Authority,
) -> Result<Option<&ThresholdConfigUpdate>, AppError> {
    match (authority, action) {
        (
            Authority::StrataAdmin,
            MultisigAction::Update(UpdateAction::StrataAdminMultisig(update)),
        ) => Ok(Some(update.config())),
        (
            Authority::SequencerManager,
            MultisigAction::Update(UpdateAction::StrataSeqManagerMultisig(update)),
        ) => Ok(Some(update.config())),
        (
            Authority::AlpenAdmin,
            MultisigAction::Update(UpdateAction::AlpenAdminMultisig(update)),
        ) => Ok(Some(update.config())),
        // Security Council rotation is authorized by the Strata Administrator, not by the
        // council — see docs/specs/security-council.md §2.1. Wired up in slice V3.
        (_, MultisigAction::Update(UpdateAction::StrataSecurityCouncilMultisig(_))) => {
            Err(AppError::BadRequest(
                "Security Council multisig update enactment is not implemented yet".to_string(),
            ))
        }
        // MultisigUpdate variant present but wrong authority — data integrity issue.
        (
            _,
            MultisigAction::Update(
                UpdateAction::StrataAdminMultisig(_)
                | UpdateAction::StrataSeqManagerMultisig(_)
                | UpdateAction::AlpenAdminMultisig(_),
            ),
        ) => Err(AppError::BadRequest(
            "action variant does not match proposal authority for enactment check".to_string(),
        )),
        // Non-multisig-update variants — not handled here; caller routes them.
        (
            _,
            MultisigAction::Update(
                UpdateAction::OperatorSet(_)
                | UpdateAction::Sequencer(_)
                | UpdateAction::OlStfVk(_)
                | UpdateAction::AsmStfVk(_)
                | UpdateAction::EeStfVk(_)
                | UpdateAction::Defcon1(_)
                | UpdateAction::Defcon3(_)
                | UpdateAction::SafeHarbourAddress(_),
            ),
        ) => Ok(None),
        (_, MultisigAction::Cancel(_)) => Err(AppError::BadRequest(
            "cancel actions are not supported for enactment post-condition checks".to_string(),
        )),
    }
}

fn multisig_update_post_conditions_met(
    canonical_keys: &[String],
    threshold: u8,
    last_seqno: u64,
    seq_no: u64,
    config: &ThresholdConfigUpdate,
) -> bool {
    if last_seqno < seq_no {
        return false;
    }
    if threshold != config.new_threshold().get() {
        return false;
    }
    for pk in config.add_members() {
        let hex_key = hex::encode(pk.serialize());
        if !canonical_keys
            .iter()
            .any(|k| k.eq_ignore_ascii_case(&hex_key))
        {
            return false;
        }
    }
    for pk in config.remove_members() {
        let hex_key = hex::encode(pk.serialize());
        if canonical_keys
            .iter()
            .any(|k| k.eq_ignore_ascii_case(&hex_key))
        {
            return false;
        }
    }
    true
}

/// Checks whether the current operator set satisfies the post-conditions of an operator set
/// update action.
///
/// - Add-only or mixed add+remove: all added keys must be present in the current set.
/// - Remove-only: heuristic — if removing index N, the original set had at least N+1 operators.
///   After removal, if the current count is <= N, at least one removal happened. This works
///   when removing from the end but may miss enactments that remove from the middle.
///   TODO: store the original operator set (or hash) in proposal metadata for reliable detection.
/// - No-op (neither add nor remove): treated as already enacted (vacuous).
fn operator_set_post_conditions_met(
    current_keys: &[String],
    add_members: &[strata_crypto::EvenPublicKey],
    remove_members: &[u32],
) -> bool {
    match (add_members.is_empty(), remove_members.is_empty()) {
        // Add-only or mixed add+remove: check all added keys are present in the current set.
        (false, _) => add_members.iter().all(|pk| {
            let key_hex = hex::encode(pk.x_only_public_key().0.serialize());
            current_keys
                .iter()
                .any(|k| k.eq_ignore_ascii_case(&key_hex))
        }),
        // Remove-only: heuristic based on the max removed index.
        (true, false) => {
            let max_remove_index = remove_members.iter().max().copied().unwrap_or(0);
            current_keys.len() as u32 <= max_remove_index
        }
        // No-op (neither add nor remove): treat as already enacted (vacuous).
        (true, true) => true,
    }
}

fn authority_to_role(authority: Authority) -> Result<Role, String> {
    match authority {
        Authority::StrataAdmin => Ok(Role::StrataAdministrator),
        Authority::SequencerManager => Ok(Role::StrataSequencerManager),
        Authority::AlpenAdmin => Ok(Role::AlpenAdministrator),
        _ => Err(format!(
            "authority `{authority:?}` is not mapped to ASM role authorization yet"
        )),
    }
}

// In-process ASM enactment mock — compiled only under `cfg(test)` or `dev-mocks`.
// In production builds this is an inert stub returning `None`, so a `mock://` URL
// never short-circuits the real enactment post-condition check.
//
// Keyed on the URL, never on the action: unlike `mock_lock_period`, which resolves a table and so
// can delegate to the real lookup, enactment is a fact about chain state that no in-process mock
// can derive. Every action — Defcon 1 included — enacts vacuously under `mock://asm-enacted`.
#[cfg(any(test, feature = "dev-mocks"))]
fn mock_is_enacted(rpc_url: &str) -> Option<bool> {
    match rpc_url {
        MOCK_ENACTED_URL => Some(true),
        MOCK_MEMBERSHIP_URL => Some(false),
        _ => None,
    }
}

#[cfg(not(any(test, feature = "dev-mocks")))]
fn mock_is_enacted(_rpc_url: &str) -> Option<bool> {
    None
}

async fn rpc_call(rpc_url: &str, method: &str, params: Value) -> Result<Value, String> {
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });

    let response = rpc_timeout::with_rpc_timeout(
        &format!("ASM RPC `{method}`"),
        client.post(rpc_url).json(&payload).send(),
    )
    .await?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "RPC method `{method}` returned unexpected status code: {status}"
        ));
    }

    let body: Value =
        rpc_timeout::with_rpc_timeout(&format!("ASM RPC `{method}` body"), response.json()).await?;

    if let Some(err) = body.get("error") {
        return Err(format!(
            "RPC method `{method}` returned JSON-RPC error: {err}"
        ));
    }

    body.get("result")
        .cloned()
        .ok_or_else(|| format!("RPC method `{method}` response does not contain `result`: {body}"))
}

fn decode_state_bytes_from_status(status_result: &Value) -> Result<Vec<u8>, String> {
    let raw_state = status_result
        .pointer("/cur_state/state")
        .or_else(|| status_result.pointer("/current_state/state"))
        .ok_or_else(|| "status result missing `cur_state.state` array".to_string())?;

    let items = raw_state
        .as_array()
        .ok_or_else(|| "`cur_state.state` is not an array".to_string())?;

    items
        .iter()
        .map(|v| {
            let n = v
                .as_u64()
                .ok_or_else(|| format!("state entry is not an unsigned integer: {v}"))?;
            u8::try_from(n).map_err(|_| format!("state entry out of byte range: {n}"))
        })
        .collect::<Result<Vec<u8>, String>>()
}

fn decode_anchor_state_from_status(status_result: &Value) -> Result<AnchorState, String> {
    let bytes = decode_state_bytes_from_status(status_result)?;
    AnchorState::from_ssz_bytes(&bytes)
        .map_err(|err| format!("failed to SSZ-decode AnchorState from status state bytes: {err}"))
}

fn decode_admin_state(anchor: &AnchorState) -> Result<AdministrationSubprotoState, String> {
    let id = AdministrationSubprotocol::ID;
    let section = anchor.find_section(id).ok_or_else(|| {
        format!("AnchorState has no administration subprotocol section (expected id {id}).")
    })?;
    section
        .try_to_state::<AdministrationSubprotocol>()
        .map_err(|e| {
            format!("Administration section (id {id}) does not decode with this app ({e:?}).")
        })
}

fn decode_bridge_state(anchor: &AnchorState) -> Result<BridgeV1State, String> {
    let id = BridgeV1Subproto::ID;
    let section = anchor.find_section(id).ok_or_else(|| {
        format!("AnchorState has no bridge-v1 subprotocol section (expected id {id})")
    })?;
    section
        .try_to_state::<BridgeV1Subproto>()
        .map_err(|e| format!("BridgeV1 section SSZ decode failed: {e:?}"))
}

fn decode_checkpoint_state(anchor: &AnchorState) -> Result<CheckpointState, String> {
    let id = CheckpointSubprotocol::ID;
    let section = anchor.find_section(id).ok_or_else(|| {
        format!("AnchorState has no checkpoint subprotocol section (expected id {id}).")
    })?;
    section
        .try_to_state::<CheckpointSubprotocol>()
        .map_err(|e| format!("Checkpoint section (id {id}) does not decode with this app ({e:?})."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU8;
    use strata_crypto::keys::compressed::CompressedPublicKey;

    fn key_hex(byte: u8) -> String {
        let mut bytes = [0u8; 33];
        bytes[0] = 0x02;
        bytes[32] = byte;
        hex::encode(bytes)
    }

    /// Generate a valid x-only public key for testing by using small scalar multiples of G.
    /// These are known valid points on secp256k1.
    fn even_pubkey_from_scalar(n: u64) -> strata_crypto::EvenPublicKey {
        use bitcoin::secp256k1::{Secp256k1, SecretKey};
        let secp = Secp256k1::new();
        let mut scalar_bytes = [0u8; 32];
        scalar_bytes[24..32].copy_from_slice(&n.to_be_bytes());
        let secret = SecretKey::from_slice(&scalar_bytes).unwrap();
        let pubkey = secret.public_key(&secp);
        let (x_only, _parity) = pubkey.x_only_public_key();
        strata_crypto::EvenPublicKey::from(x_only)
    }

    fn even_key_hex_from_scalar(n: u64) -> String {
        hex::encode(even_pubkey_from_scalar(n).x_only_public_key().0.serialize())
    }

    #[test]
    fn post_conditions_require_last_seqno_at_least_proposal_seq() {
        let config = ThresholdConfigUpdate::new(vec![], vec![], NonZeroU8::new(2).unwrap());
        let keys = vec![key_hex(1), key_hex(2)];
        assert!(!multisig_update_post_conditions_met(
            &keys, 2, 0, 1, &config
        ));
        assert!(multisig_update_post_conditions_met(&keys, 2, 1, 1, &config));
    }

    #[test]
    fn post_conditions_require_added_keys_present_and_removed_absent() {
        let added = CompressedPublicKey::from_slice(&hex::decode(key_hex(3)).unwrap()).unwrap();
        let removed = CompressedPublicKey::from_slice(&hex::decode(key_hex(2)).unwrap()).unwrap();
        let config =
            ThresholdConfigUpdate::new(vec![added], vec![removed], NonZeroU8::new(2).unwrap());

        let before = vec![key_hex(1), key_hex(2)];
        assert!(!multisig_update_post_conditions_met(
            &before, 2, 1, 1, &config
        ));

        let after = vec![key_hex(1), key_hex(3)];
        assert!(multisig_update_post_conditions_met(
            &after, 2, 1, 1, &config
        ));
    }

    #[test]
    fn operator_set_add_only_enacted_when_keys_present() {
        let pk = even_pubkey_from_scalar(1);
        let current = vec![even_key_hex_from_scalar(1), even_key_hex_from_scalar(2)];
        assert!(operator_set_post_conditions_met(&current, &[pk], &[]));
    }

    #[test]
    fn operator_set_add_only_not_enacted_when_keys_missing() {
        let pk = even_pubkey_from_scalar(3);
        let current = vec![even_key_hex_from_scalar(1), even_key_hex_from_scalar(2)];
        assert!(!operator_set_post_conditions_met(&current, &[pk], &[]));
    }

    #[test]
    fn operator_set_remove_only_enacted_when_count_shrunk() {
        // Removing index 2 means original had at least 3 operators.
        // After removal, if current count is <= 2, enactment is detected.
        let current = vec![even_key_hex_from_scalar(1), even_key_hex_from_scalar(2)];
        assert!(operator_set_post_conditions_met(&current, &[], &[2]));
    }

    #[test]
    fn operator_set_remove_only_not_enacted_when_count_unchanged() {
        // Removing index 2 means original had at least 3 operators.
        // If current count is still 3, removal hasn't happened yet.
        let current = vec![
            even_key_hex_from_scalar(1),
            even_key_hex_from_scalar(2),
            even_key_hex_from_scalar(3),
        ];
        assert!(!operator_set_post_conditions_met(&current, &[], &[2]));
    }

    #[test]
    fn operator_set_no_op_is_vacuously_enacted() {
        let current = vec![even_key_hex_from_scalar(1)];
        assert!(operator_set_post_conditions_met(&current, &[], &[]));
    }

    #[test]
    fn defcon1_enacted_requires_safe_harbour_active_and_queue_clear() {
        assert!(!defcon1_enacted(false, false));
        assert!(!defcon1_enacted(true, true));
        assert!(defcon1_enacted(true, false));
    }

    #[test]
    fn ee_stf_vk_enacted_requires_seqno_consumed_and_not_queued() {
        assert!(!ee_stf_vk_enacted(2, 3, false));
        assert!(!ee_stf_vk_enacted(3, 3, true));
        assert!(ee_stf_vk_enacted(3, 3, false));
    }

    /// `UpdateAction::Sequencer` enactment is detected by comparing the proposed key
    /// (wrapped as `Bip340Schnorr` predicate) against `checkpoint.sequencer_predicate()`.
    #[test]
    fn sequencer_predicate_keys_match_detects_enactment() {
        let key_bytes = [0x03u8; 32];
        let matching = PredicateKey::new(PredicateTypeId::Bip340Schnorr, key_bytes.to_vec());
        let different = PredicateKey::new(PredicateTypeId::Bip340Schnorr, vec![0x04u8; 32]);

        assert!(predicate_keys_match(&matching, &matching));
        assert!(!predicate_keys_match(&matching, &different));
    }
}
