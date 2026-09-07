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
use crate::infrastructure::{action_codec, asm_role_membership, http_client, rpc_timeout};

#[cfg(any(test, feature = "dev-mocks"))]
const MOCK_MEMBERSHIP_URL: &str = "mock://asm-membership";
#[cfg(any(test, feature = "dev-mocks"))]
pub(crate) const MOCK_ENACTED_URL: &str = "mock://asm-enacted";
/// Enacts *and* stands past the proposal's seqno: the fixture that proves enactment is decided
/// before supersession.
#[cfg(any(test, feature = "dev-mocks"))]
pub(crate) const MOCK_ENACTED_AHEAD_URL: &str = "mock://asm-enacted-ahead";
/// A chain whose role sequence number has moved past the proposals under test: nothing enacts, and
/// `last_seqno` answers 5. The fixture for supersession.
#[cfg(any(test, feature = "dev-mocks"))]
pub(crate) const MOCK_SEQNO_AHEAD_URL: &str = "mock://asm-seqno-ahead";

/// Returns true when live ASM canonical state satisfies the post-conditions of `action_hex`.
///
/// `activation_height` and `bitcoin_tip` are only read by the Defcon 3 arm. Other variants ignore
/// them. Missing values there are inconclusive (`Err`), not "not enacted" — `Ok(false)` would fall
/// through to supersession.
pub(crate) async fn is_proposal_enacted_on_asm(
    rpc_url: &str,
    authority: Authority,
    seq_no: u64,
    action_hex: &str,
    activation_height: Option<u64>,
    bitcoin_tip: Option<u64>,
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
        // and Defcon 3 have post-conditions; SafeHarbourAddress waits on slice V4. See
        // docs/specs/security-council.md and docs/specs/security-council-defcon-3-phase-4.md.
        MultisigAction::Update(UpdateAction::Defcon1(_)) => {
            let bridge = decode_bridge_state(&anchor).map_err(AppError::BadRequest)?;
            let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;
            let safe_harbour_activated = bridge.safe_harbour().is_activated();
            let defcon1_queued = admin
                .queued()
                .iter()
                .any(|q| matches!(q.action(), UpdateAction::Defcon1(_)));
            // The role is named literally: an arm that matches one action variant knows its role.
            let council = admin
                .authority(Role::StrataSecurityCouncil)
                .ok_or_else(|| {
                    AppError::BadRequest(
                        "admin state missing authority for role `StrataSecurityCouncil`"
                            .to_string(),
                    )
                })?;
            Ok(defcon1_enacted(
                safe_harbour_activated,
                defcon1_queued,
                council.last_seqno(),
                seq_no,
            ))
        }
        MultisigAction::Update(UpdateAction::Defcon3(_)) => {
            let (activation_height, bitcoin_tip) =
                defcon3_observations(activation_height, bitcoin_tip)?;
            let bridge = decode_bridge_state(&anchor).map_err(AppError::BadRequest)?;
            let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;
            // Payload is empty, so this is the same question as equality against `this` action —
            // two in-flight Defcon 3s share queue state (contract edge case). Same shape as Defcon 1.
            let still_queued = admin
                .queued()
                .iter()
                .any(|q| matches!(q.action(), UpdateAction::Defcon3(_)));
            let council = admin
                .authority(Role::StrataSecurityCouncil)
                .ok_or_else(|| {
                    AppError::BadRequest(
                        "admin state missing authority for role `StrataSecurityCouncil`"
                            .to_string(),
                    )
                })?;
            Ok(defcon3_enacted(
                council.last_seqno(),
                seq_no,
                still_queued,
                bridge.safe_harbour().is_activated(),
                bitcoin_tip,
                activation_height,
            ))
        }
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
            // The target lookup runs before the authorization guard: reversed, an `AsmStfVk`
            // under a non-administrator authority would go from `Ok(false)` to `Err`, and
            // `reconcile_one` turns every `Err` into a per-proposal warning that never resolves.
            // See docs/specs/security-council-signer-update-phase-2.md §10.3.
            let Some((target_role, config_update)) = multisig_config_update_target(&action) else {
                return Ok(false);
            };
            asm_role_membership::require_authorized_for_action(authority, &action)?;
            let authorizing_role = match &action {
                MultisigAction::Update(update) => update.required_role(),
                _ => unreachable!("outer arm already matched MultisigAction::Update"),
            };
            let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;

            multisig_update_enacted(
                target_role,
                authorizing_role,
                seq_no,
                config_update,
                |role| {
                    let authority_config = admin.authority(role)?;
                    Some(AuthoritySnapshot {
                        keys: authority_config
                            .config()
                            .keys()
                            .iter()
                            .map(|k| hex::encode(k.serialize()))
                            .collect(),
                        threshold: authority_config.config().threshold(),
                        last_seqno: authority_config.last_seqno(),
                    })
                },
            )
            .map_err(AppError::BadRequest)
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
///
/// The seqno term is what makes the answer this proposal's: `safe_harbour().is_activated()` is
/// never reset, so the other two terms hold for every Defcon 1 once any of them has enacted.
///
/// It is an equality, not `>=`, and this arm is the only one in the module that needs it.
/// `update_last_seqno` jumps to whatever seqno upstream accepted, for any action of the role, so
/// `>=` asks "has the role moved past this point" — a question another proposal can answer. `==`
/// asks whether the role is standing exactly where this proposal would have left it. The
/// config-carrying arms keep `>=` because their remaining terms name the keys, threshold or VK the
/// action was supposed to install, which a jumped seqno does not supply.
///
/// See docs/specs/proposal-lifecycle-seqno-truth.md §3.1 and §4.1, including what stays ambiguous.
fn defcon1_enacted(
    safe_harbour_activated: bool,
    defcon1_queued: bool,
    last_seqno: u64,
    seq_no: u64,
) -> bool {
    last_seqno == seq_no && safe_harbour_activated && !defcon1_queued
}

/// Defcon 3 matures after `activation_height` blocks. A cancel removes the queue entry before that
/// height; the tip term is what separates "matured" from "taken out early".
///
/// Uses `>=` on the seqno, not `==`: upstream consumes the seqno at reveal acceptance, and a
/// later council action may jump `last_seqno` past this proposal before it matures. Equality would
/// leave a successfully enacted Defcon 3 marked `Superseded`. See Constraint 2 in
/// docs/specs/security-council-defcon-3.md.
fn defcon3_enacted(
    last_seqno: u64,
    seq_no: u64,
    still_queued: bool,
    safe_harbour_activated: bool,
    bitcoin_tip: u64,
    activation_height: u64,
) -> bool {
    last_seqno >= seq_no
        && !still_queued
        && safe_harbour_activated
        && bitcoin_tip >= activation_height
}

/// Missing height or tip is inconclusive: the caller must not treat that as `not enacted`.
///
/// `Conflict`, not `BadRequest`: nothing about the request is wrong — the answer is not available
/// yet. The reconciliation cycle logs any `Err` and retries; the broadcast endpoint already answers
/// a not-yet-enacted proposal with the same status.
fn defcon3_observations(
    activation_height: Option<u64>,
    bitcoin_tip: Option<u64>,
) -> Result<(u64, u64), AppError> {
    let activation_height = activation_height.ok_or_else(|| {
        AppError::Conflict(
            "Defcon 3 enactment is inconclusive: activation_height is missing".to_string(),
        )
    })?;
    let bitcoin_tip = bitcoin_tip.ok_or_else(|| {
        AppError::Conflict(
            "Defcon 3 enactment is inconclusive: bitcoin tip is unavailable".to_string(),
        )
    })?;
    Ok((activation_height, bitcoin_tip))
}

/// Defcon 3 is the only arm that compares against chain tip. Other actions must not pay that RPC.
pub(crate) fn action_needs_chain_tip(action_hex: &str) -> bool {
    matches!(
        action_codec::decode_multisig_action_hex(action_hex),
        Ok(MultisigAction::Update(UpdateAction::Defcon3(_)))
    )
}

/// The role a multisig-config update *modifies*, and the config it installs.
///
/// The target belongs to the action variant and to nothing else — see Constraint 2. Upstream
/// applies tx type 15 to `Role::StrataSecurityCouncil` (`handler.rs:145-147`) while authorizing it
/// with `Role::StrataAdministrator` (`updates.rs:64`); for the three self-rotating updates the two
/// coincide, which is why nothing needed this distinction before V3.
///
/// `None` for every action that is not a multisig config update — the caller answers `Ok(false)`,
/// which is what `AsmStfVk` has always relied on.
fn multisig_config_update_target(
    action: &MultisigAction,
) -> Option<(Role, &ThresholdConfigUpdate)> {
    let MultisigAction::Update(update) = action else {
        return None;
    };
    match update {
        UpdateAction::StrataAdminMultisig(update) => {
            Some((Role::StrataAdministrator, update.config()))
        }
        UpdateAction::StrataSeqManagerMultisig(update) => {
            Some((Role::StrataSequencerManager, update.config()))
        }
        UpdateAction::AlpenAdminMultisig(update) => {
            Some((Role::AlpenAdministrator, update.config()))
        }
        UpdateAction::StrataSecurityCouncilMultisig(update) => {
            Some((Role::StrataSecurityCouncil, update.config()))
        }
        UpdateAction::OperatorSet(_)
        | UpdateAction::Sequencer(_)
        | UpdateAction::OlStfVk(_)
        | UpdateAction::AsmStfVk(_)
        | UpdateAction::EeStfVk(_)
        | UpdateAction::Defcon1(_)
        | UpdateAction::Defcon3(_)
        | UpdateAction::SafeHarbourAddress(_) => None,
    }
}

/// The three terms an enactment check reads off one role, at one instant.
///
/// `keys` is hex of `CompressedPublicKey::serialize()` — 33 bytes, compressed. Not x-only: the
/// `OperatorSet` arm next door uses 32-byte x-only hex, and the two are not interchangeable.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthoritySnapshot {
    keys: Vec<String>,
    threshold: u8,
    last_seqno: u64,
}

/// `keys` and `threshold` come from the **target** role; `last_seqno` from the **authorizing**
/// role. Neither term is derived from the other, and collapsing the two roles into one is the
/// regression AC 7a exists to catch.
///
/// `snapshot_of` is a parameter for the same reason `depth_for_action` takes its lookup: it is the
/// only seam at which the two-role wiring is observable without a chain.
///
/// A role the state does not carry is `Err`, never `Ok(false)` — see §6.
fn multisig_update_enacted(
    target_role: Role,
    authorizing_role: Role,
    seq_no: u64,
    config: &ThresholdConfigUpdate,
    snapshot_of: impl Fn(Role) -> Option<AuthoritySnapshot>,
) -> Result<bool, String> {
    let target = snapshot_of(target_role)
        .ok_or_else(|| format!("admin state missing authority for role `{target_role:?}`"))?;
    let authorizing = snapshot_of(authorizing_role)
        .ok_or_else(|| format!("admin state missing authority for role `{authorizing_role:?}`"))?;

    Ok(multisig_update_post_conditions_met(
        &target.keys,
        target.threshold,
        authorizing.last_seqno,
        seq_no,
        config,
    ))
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
        MOCK_ENACTED_URL | MOCK_ENACTED_AHEAD_URL => Some(true),
        MOCK_MEMBERSHIP_URL | MOCK_SEQNO_AHEAD_URL => Some(false),
        _ => None,
    }
}

#[cfg(not(any(test, feature = "dev-mocks")))]
fn mock_is_enacted(_rpc_url: &str) -> Option<bool> {
    None
}

async fn rpc_call(rpc_url: &str, method: &str, params: Value) -> Result<Value, String> {
    let client = http_client::shared();
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

    /// A `snapshot_of` for exactly two roles — the simplest lookup that still lets a test say
    /// "this role is present, that one isn't".
    fn snapshot_of_two(
        role_a: Role,
        snap_a: AuthoritySnapshot,
        role_b: Role,
        snap_b: AuthoritySnapshot,
    ) -> impl Fn(Role) -> Option<AuthoritySnapshot> {
        move |role| {
            if role == role_a {
                Some(snap_a.clone())
            } else if role == role_b {
                Some(snap_b.clone())
            } else {
                None
            }
        }
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

    /// The seqno term is what makes the answer per proposal: `safe_harbour().is_activated()` is
    /// never reset, so without it every later Defcon 1 reads as enacted on the strength of the
    /// first one's activation.
    #[test]
    fn defcon1_enacted_requires_this_proposals_seqno_consumed() {
        assert!(!defcon1_enacted(true, false, 1, 2));
        assert!(defcon1_enacted(true, false, 2, 2));
    }

    /// Upstream jumps `last_seqno` to whatever it accepted, so a role standing past this
    /// proposal's seqno is another action's doing, not this one's. Equality is the whole
    /// difference between "the role moved on" and "this proposal moved it".
    #[test]
    fn defcon1_not_enacted_when_a_later_action_consumed_the_seqno() {
        assert!(!defcon1_enacted(true, false, 2, 1));
    }

    #[test]
    fn defcon1_enacted_requires_safe_harbour_active_and_queue_clear() {
        assert!(!defcon1_enacted(false, false, 2, 2));
        assert!(!defcon1_enacted(true, true, 2, 2));
        assert!(defcon1_enacted(true, false, 2, 2));
    }

    #[test]
    fn ee_stf_vk_enacted_requires_seqno_consumed_and_not_queued() {
        assert!(!ee_stf_vk_enacted(2, 3, false));
        assert!(!ee_stf_vk_enacted(3, 3, true));
        assert!(ee_stf_vk_enacted(3, 3, false));
    }

    /// Constraint 2: the seqno term is `>=`. Defcon 1 answers the same observation with `==` and
    /// says "not enacted" — which for a Defcon 3 would end in `Superseded`.
    #[test]
    fn defcon3_enacted_when_a_later_action_consumed_the_seqno() {
        assert!(defcon3_enacted(5, 2, false, true, 120, 100));
        assert!(!defcon1_enacted(true, false, 5, 2));
    }

    #[test]
    fn defcon3_not_enacted_when_seqno_still_below() {
        assert!(!defcon3_enacted(1, 2, false, true, 120, 100));
    }

    #[test]
    fn defcon3_not_enacted_while_still_queued() {
        assert!(!defcon3_enacted(2, 2, true, true, 120, 100));
    }

    #[test]
    fn defcon3_not_enacted_when_harbour_off() {
        assert!(!defcon3_enacted(2, 2, false, false, 120, 100));
    }

    #[test]
    fn defcon3_not_enacted_before_activation_height() {
        assert!(!defcon3_enacted(2, 2, false, true, 99, 100));
    }

    #[test]
    fn defcon3_enacted_at_exact_activation_height() {
        assert!(defcon3_enacted(2, 2, false, true, 100, 100));
    }

    #[test]
    fn defcon3_missing_observations_are_inconclusive() {
        assert!(defcon3_observations(None, Some(100)).is_err());
        assert!(defcon3_observations(Some(100), None).is_err());
        assert!(defcon3_observations(Some(100), Some(120)).is_ok());
    }

    #[test]
    fn only_defcon_3_needs_chain_tip() {
        assert!(action_needs_chain_tip(
            &crate::infrastructure::action_codec::test_fixture_defcon_3_action_hex()
        ));
        assert!(!action_needs_chain_tip(
            &crate::infrastructure::action_codec::test_fixture_defcon_1_action_hex()
        ));
        assert!(!action_needs_chain_tip("deadbeef"));
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

    /// AC 7 — the happy path, and nothing else.
    ///
    /// Both snapshots satisfy every term deliberately: the administrator's keys and threshold
    /// agree with `config`, and both roles stand at the same `last_seqno`. So this test answers
    /// `true` under the real wiring *and* under either role substitution, which is what makes it a
    /// control rather than a second copy of the two that follow. A fixture where the two roles
    /// happen to disagree would let this one test catch both swaps, and the named tests below
    /// would then be asserting something already proven — see this phase's spec §8.
    #[test]
    fn council_rotation_targets_the_council_and_the_administrator_authorizes_it() {
        let added = CompressedPublicKey::from_slice(&hex::decode(key_hex(3)).unwrap()).unwrap();
        let config = ThresholdConfigUpdate::new(vec![added], vec![], NonZeroU8::new(3).unwrap());

        let council = AuthoritySnapshot {
            keys: vec![key_hex(1), key_hex(3)],
            threshold: 3,
            last_seqno: 1,
        };
        let administrator = AuthoritySnapshot {
            keys: vec![key_hex(1), key_hex(3)],
            threshold: 3,
            last_seqno: 1,
        };
        let snapshot_of = snapshot_of_two(
            Role::StrataSecurityCouncil,
            council,
            Role::StrataAdministrator,
            administrator,
        );

        assert_eq!(
            multisig_update_enacted(
                Role::StrataSecurityCouncil,
                Role::StrataAdministrator,
                1,
                &config,
                snapshot_of,
            ),
            Ok(true)
        );
    }

    /// AC 7a, half one — and the **only** test that goes red if `keys` and `threshold` are ever
    /// read off the authorizing role. The administrator's set disagrees with `config` on both
    /// terms, so the swap answers `false` where `true` is expected.
    ///
    /// Both roles stand at the same `last_seqno` on purpose: that is the term the other half owns,
    /// and leaving it able to fail here would make the two tests fail together and stop telling a
    /// reader which substitution happened.
    #[test]
    fn council_rotation_ignores_the_administrators_signer_set() {
        let added = CompressedPublicKey::from_slice(&hex::decode(key_hex(3)).unwrap()).unwrap();
        let config = ThresholdConfigUpdate::new(vec![added], vec![], NonZeroU8::new(3).unwrap());

        let council = AuthoritySnapshot {
            keys: vec![key_hex(1), key_hex(3)],
            threshold: 3,
            last_seqno: 1,
        };
        let administrator = AuthoritySnapshot {
            keys: vec![key_hex(5)],
            threshold: 7,
            last_seqno: 1,
        };
        let snapshot_of = snapshot_of_two(
            Role::StrataSecurityCouncil,
            council,
            Role::StrataAdministrator,
            administrator,
        );

        assert_eq!(
            multisig_update_enacted(
                Role::StrataSecurityCouncil,
                Role::StrataAdministrator,
                1,
                &config,
                snapshot_of,
            ),
            Ok(true)
        );
    }

    /// AC 7a, half two — and the test that fails if a future refactor collapses the two roles
    /// back into one, which is the shape the code had before this phase. The council's own
    /// `last_seqno` races ahead of `seq_no`; only the administrator's `last_seqno` may decide
    /// whether the rotation was authorized.
    #[test]
    fn council_rotation_ignores_the_councils_own_seqno() {
        let added = CompressedPublicKey::from_slice(&hex::decode(key_hex(3)).unwrap()).unwrap();
        let config = ThresholdConfigUpdate::new(vec![added], vec![], NonZeroU8::new(3).unwrap());

        let council = AuthoritySnapshot {
            keys: vec![key_hex(1), key_hex(3)],
            threshold: 3,
            last_seqno: 100,
        };
        let administrator = AuthoritySnapshot {
            keys: vec![key_hex(2)],
            threshold: 1,
            last_seqno: 1,
        };
        let snapshot_of = snapshot_of_two(
            Role::StrataSecurityCouncil,
            council,
            Role::StrataAdministrator,
            administrator,
        );

        assert_eq!(
            multisig_update_enacted(
                Role::StrataSecurityCouncil,
                Role::StrataAdministrator,
                5,
                &config,
                snapshot_of,
            ),
            Ok(false)
        );
    }

    /// The three authorities shipped before V3 self-rotate: target and authorizing role coincide,
    /// so reading both from one role must answer exactly as it always did.
    #[test]
    fn an_administrator_signer_update_reads_one_role_for_all_three_terms() {
        let added = CompressedPublicKey::from_slice(&hex::decode(key_hex(3)).unwrap()).unwrap();
        let config = ThresholdConfigUpdate::new(vec![added], vec![], NonZeroU8::new(2).unwrap());

        let administrator = AuthoritySnapshot {
            keys: vec![key_hex(1), key_hex(3)],
            threshold: 2,
            last_seqno: 1,
        };
        let snapshot_of =
            move |role: Role| (role == Role::StrataAdministrator).then(|| administrator.clone());

        assert_eq!(
            multisig_update_enacted(
                Role::StrataAdministrator,
                Role::StrataAdministrator,
                1,
                &config,
                snapshot_of,
            ),
            Ok(true)
        );
    }

    /// §6: a role the state does not carry is an absence, never a negative answer — `Ok(false)`
    /// here would tell `reconcile_one` to supersede a rotation that may still be live.
    #[test]
    fn a_missing_target_authority_is_an_error_not_a_negative_answer() {
        let config = ThresholdConfigUpdate::new(vec![], vec![], NonZeroU8::new(2).unwrap());
        let administrator = AuthoritySnapshot {
            keys: vec![key_hex(1)],
            threshold: 1,
            last_seqno: 1,
        };
        // The council — the target — is absent; the administrator — the authorizer — is present.
        let snapshot_of =
            move |role: Role| (role == Role::StrataAdministrator).then(|| administrator.clone());

        let result = multisig_update_enacted(
            Role::StrataSecurityCouncil,
            Role::StrataAdministrator,
            1,
            &config,
            snapshot_of,
        );
        assert!(result.is_err());
    }

    /// `AsmStfVk` rides in the multisig arm by inheritance but has no config to target — the
    /// caller must keep answering `Ok(false)` for it, not fall into this lookup.
    #[test]
    fn asm_stf_vk_is_not_a_multisig_config_update() {
        use strata_asm_txs_admin::actions::updates::AsmStfVkUpdate;
        use strata_predicate::{PredicateKey, PredicateTypeId};

        let action = MultisigAction::Update(UpdateAction::AsmStfVk(AsmStfVkUpdate::new(
            PredicateKey::new(PredicateTypeId::Bip340Schnorr, vec![0u8; 32]),
        )));
        assert_eq!(multisig_config_update_target(&action), None);
    }

    /// Not restating the enum: a codec with two arms crossed would still round-trip, so this
    /// pins the role each variant names, council included, rather than merely that four exist.
    #[test]
    fn every_multisig_variant_names_its_own_target_role() {
        use strata_asm_txs_admin::actions::updates::{
            AlpenAdminMultisigUpdate, StrataAdminMultisigUpdate,
            StrataSecurityCouncilMultisigUpdate, StrataSeqManagerMultisigUpdate,
        };

        let config = || ThresholdConfigUpdate::new(vec![], vec![], NonZeroU8::new(2).unwrap());

        let cases = [
            (
                MultisigAction::Update(UpdateAction::StrataAdminMultisig(
                    StrataAdminMultisigUpdate::new(config()),
                )),
                Role::StrataAdministrator,
            ),
            (
                MultisigAction::Update(UpdateAction::StrataSeqManagerMultisig(
                    StrataSeqManagerMultisigUpdate::new(config()),
                )),
                Role::StrataSequencerManager,
            ),
            (
                MultisigAction::Update(UpdateAction::AlpenAdminMultisig(
                    AlpenAdminMultisigUpdate::new(config()),
                )),
                Role::AlpenAdministrator,
            ),
            (
                MultisigAction::Update(UpdateAction::StrataSecurityCouncilMultisig(
                    StrataSecurityCouncilMultisigUpdate::new(config()),
                )),
                Role::StrataSecurityCouncil,
            ),
        ];

        for (action, expected_role) in &cases {
            let (role, _) = multisig_config_update_target(action).expect("multisig config update");
            assert_eq!(role, *expected_role);
        }
    }
}
