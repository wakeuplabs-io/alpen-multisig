//! Coordination hygiene: detect when ASM canonical state reflects a proposal's governance change.

use serde_json::{json, Value};
use ssz::Decode;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_params::Role;
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};
use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
use strata_crypto::threshold_signature::ThresholdConfigUpdate;

use crate::domain::authority::Authority;
#[cfg(any(test, feature = "dev-mocks"))]
const MOCK_MEMBERSHIP_URL: &str = "mock://asm-membership";
#[cfg(any(test, feature = "dev-mocks"))]
const MOCK_ENACTED_URL: &str = "mock://asm-enacted";

/// Returns true when admin state satisfies the post-conditions of `action_hex`.
pub fn is_multisig_update_enacted_in_admin_state(
    admin: &AdministrationSubprotoState,
    authority: Authority,
    seq_no: u64,
    action_hex: &str,
) -> Result<bool, String> {
    let action_bytes =
        hex::decode(action_hex.trim()).map_err(|e| format!("invalid action hex: {e}"))?;
    let action = MultisigAction::from_ssz_bytes(&action_bytes)
        .map_err(|e| format!("invalid SSZ MultisigAction: {e:?}"))?;

    // A cancel carries no config to compare against, and answering `Ok(false)` for one would be
    // an answer where there is none — the direction this module must never take, since a caller
    // reads `Ok(false)` as "not enacted" and retires the proposal on it.
    let MultisigAction::Update(update) = &action else {
        return Err(
            "cancel actions are not supported for enactment post-condition checks".to_string(),
        );
    };

    // The target lookup runs before the authorization guard: reversed, an `AsmStfVk` under an
    // authority that does not authorize it would go from `Ok(false)` to `Err`.
    // See docs/specs/security-council-signer-update-phase-2.md §10.3.
    let Some((target_role, config_update)) = multisig_config_update_target(&action) else {
        return Ok(false);
    };

    let authorizing_role = update.required_role();
    let session_role = authority_to_role(authority)?;
    if session_role != authorizing_role {
        return Err(format!(
            "action `{}` must be authorized by `{authorizing_role}`, but the session is `{session_role}`",
            update.update_tx_type().name(),
        ));
    }

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
}

/// Returns true when live ASM admin state satisfies the post-conditions of `action_hex`.
pub async fn is_proposal_enacted_on_asm(
    rpc_url: &str,
    authority: Authority,
    seq_no: u64,
    action_hex: &str,
) -> Result<bool, String> {
    if let Some(enacted) = mock_is_enacted(rpc_url) {
        return Ok(enacted);
    }
    super::reject_mock_asm_url_in_prod(rpc_url)?;

    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([])).await?;
    let anchor = decode_anchor_state_from_status(&status_result)?;
    let admin = decode_admin_state(&anchor)?;
    is_multisig_update_enacted_in_admin_state(&admin, authority, seq_no, action_hex)
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

fn authority_to_role(authority: Authority) -> Result<Role, String> {
    // Exhaustive for the same reason as its twin in `asm_role_membership`: the council sat in a
    // catch-all error arm here for four phases, and a new authority upstream should stop the
    // build rather than surface as a runtime failure.
    match authority {
        Authority::StrataAdmin => Ok(Role::StrataAdministrator),
        Authority::SequencerManager => Ok(Role::StrataSequencerManager),
        Authority::AlpenAdmin => Ok(Role::AlpenAdministrator),
        Authority::SecurityCouncil => Ok(Role::StrataSecurityCouncil),
        // No ASM role upstream.
        Authority::PayoutAdmin => Err(format!(
            "authority `{authority:?}` is not mapped to ASM role authorization yet"
        )),
    }
}

// In-process ASM enactment mock — compiled only under `cfg(test)` or `dev-mocks`.
// In production builds this is an inert stub returning `None`.
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
    let client = super::rpc_timeout::rpc_client();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });

    let response = client
        .post(rpc_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("rpc send failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "RPC method `{method}` returned unexpected status code: {status}"
        ));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("invalid rpc json body: {e}"))?;

    if let Some(err) = body.get("error") {
        return Err(format!(
            "RPC method `{method}` returned JSON-RPC error: {err}"
        ));
    }

    body.get("result")
        .cloned()
        .ok_or_else(|| format!("RPC method `{method}` response does not contain `result`: {body}"))
}

fn decode_anchor_state_from_status(status_result: &Value) -> Result<AnchorState, String> {
    let raw_state = status_result
        .pointer("/cur_state/state")
        .or_else(|| status_result.pointer("/current_state/state"))
        .ok_or_else(|| "status result missing `cur_state.state` array".to_string())?;

    let items = raw_state
        .as_array()
        .ok_or_else(|| "`cur_state.state` is not an array".to_string())?;

    let bytes: Vec<u8> = items
        .iter()
        .map(|v| {
            let n = v
                .as_u64()
                .ok_or_else(|| format!("state entry is not an unsigned integer: {v}"))?;
            u8::try_from(n).map_err(|_| format!("state entry out of byte range: {n}"))
        })
        .collect::<Result<Vec<u8>, String>>()?;

    AnchorState::from_ssz_bytes(&bytes)
        .map_err(|err| format!("failed to SSZ-decode AnchorState: {err}"))
}

fn decode_admin_state(anchor: &AnchorState) -> Result<AdministrationSubprotoState, String> {
    let id = AdministrationSubprotocol::ID;
    let section = anchor.find_section(id).ok_or_else(|| {
        format!("AnchorState has no administration subprotocol section (id {id})")
    })?;
    section
        .try_to_state::<AdministrationSubprotocol>()
        .map_err(|e| format!("administration section does not decode ({e:?})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU8;

    use crate::infrastructure::action_codec;
    use strata_asm_params::{AdministrationInitConfig, ConfirmationDepths};
    use strata_asm_txs_admin::actions::updates::{AlpenAdminMultisigUpdate, AsmStfVkUpdate};
    use strata_asm_txs_admin::actions::CancelAction;
    use strata_crypto::keys::compressed::CompressedPublicKey;

    fn key_hex(byte: u8) -> String {
        let mut bytes = [0u8; 33];
        bytes[0] = 0x02;
        bytes[32] = byte;
        hex::encode(bytes)
    }

    /// A four-role admin state with one signer and threshold 1 everywhere, built through
    /// upstream's own constructors — real state, no mock, no chain, no I/O.
    ///
    /// Every `last_seqno` in it is zero and cannot be anything else: upstream's
    /// `update_last_seqno` demands a token no outside crate can construct. That is exactly why
    /// the two-role tests above go through the `multisig_update_enacted` seam instead, and why
    /// the tests using this helper assert about wiring rather than about sequence numbers.
    fn single_signer_admin_state() -> AdministrationSubprotoState {
        let key = CompressedPublicKey::from_slice(&hex::decode(key_hex(1)).unwrap()).unwrap();
        let threshold = NonZeroU8::new(1).unwrap();
        let single_signer_config = || {
            strata_crypto::threshold_signature::ThresholdConfig::try_new(vec![key], threshold)
                .unwrap()
        };

        AdministrationSubprotoState::new(&AdministrationInitConfig::new(
            single_signer_config(),
            single_signer_config(),
            single_signer_config(),
            single_signer_config(),
            ConfirmationDepths {
                strata_admin_multisig_update: 0,
                strata_seq_manager_multisig_update: 0,
                alpen_admin_multisig_update: 0,
                strata_security_council_multisig_update: 0,
                operator_update: 0,
                sequencer_update: 0,
                ol_stf_vk_update: 0,
                asm_stf_vk_update: 0,
                ee_stf_vk_update: 0,
                defcon3: 0,
                safe_harbour_address_update: 0,
            },
            NonZeroU8::new(1).unwrap(),
        ))
    }

    /// A `snapshot_of` for exactly two roles — the simplest lookup that still lets a test say
    /// "this role is present, that one isn't". Mirrors the backend's helper of the same name.
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

    /// T8, AC 7 half — the desktop copy wires the same two roles as the backend: keys/threshold
    /// from the target, `last_seqno` from the authorizer.
    #[test]
    fn council_rotation_targets_the_council_and_the_administrator_authorizes_it() {
        let added = CompressedPublicKey::from_slice(&hex::decode(key_hex(3)).unwrap()).unwrap();
        let config = ThresholdConfigUpdate::new(vec![added], vec![], NonZeroU8::new(3).unwrap());

        let council = AuthoritySnapshot {
            keys: vec![key_hex(1), key_hex(3)],
            threshold: 3,
            last_seqno: 0,
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
                1,
                &config,
                snapshot_of,
            ),
            Ok(true)
        );
    }

    /// T8, AC 7a half two — and the test that fails if a future refactor collapses the two roles
    /// back into one. The council's own `last_seqno` races ahead of `seq_no`; only the
    /// administrator's `last_seqno` may decide whether the rotation was authorized.
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

    /// The bug this file shipped with, made concrete. `extract_multisig_config_update` mapped
    /// only `StrataAdmin` and `SequencerManager`, so an `AlpenAdminMultisig` update authored
    /// under `Authority::AlpenAdmin` matched neither of the two real arms nor the mismatch arm
    /// beside them — it fell through to `(_, MultisigAction::Update(_)) => Ok(None)`, and this
    /// predicate answered `Ok(false)` for it. Permanently, and without complaint.
    ///
    /// `Ok(false)` is the worse of the two failures: an `Err` is logged and retried, while
    /// `Ok(false)` is an answer a caller acts on. Resolving the target from the variant alone is
    /// what makes the same proposal answer the real question.
    #[test]
    fn alpen_admin_rotation_is_no_longer_refused() {
        let admin = single_signer_admin_state();

        // A no-op relative to the config the state was built with: add nothing, remove nothing,
        // same threshold. Only the wiring is under test, not the arithmetic.
        let config_update = ThresholdConfigUpdate::new(vec![], vec![], NonZeroU8::new(1).unwrap());
        let action = MultisigAction::Update(UpdateAction::AlpenAdminMultisig(
            AlpenAdminMultisigUpdate::new(config_update),
        ));
        let action_hex = hex::encode(ssz::Encode::as_ssz_bytes(&action));

        let result = is_multisig_update_enacted_in_admin_state(
            &admin,
            Authority::AlpenAdmin,
            0,
            &action_hex,
        );

        assert_eq!(result, Ok(true));
    }

    /// The target lookup runs before the authorization guard, and only a test can hold that
    /// order in place — swapping the two lines compiles, passes every other test, and turns this
    /// case from `Ok(false)` into `Err`. A caller logs an `Err` and retries it forever, so the
    /// swap costs a per-proposal warning that never resolves. See this phase's spec §10.3.
    ///
    /// `AsmStfVk` is the variant that reaches the guard without being a multisig config update,
    /// and `SecurityCouncil` is an authority that does not authorize it.
    #[test]
    fn an_action_that_is_not_a_multisig_update_answers_before_the_guard() {
        let action = MultisigAction::Update(UpdateAction::AsmStfVk(AsmStfVkUpdate::new(
            strata_predicate::PredicateKey::new(
                strata_predicate::PredicateTypeId::Bip340Schnorr,
                vec![7u8; 32],
            ),
        )));
        let action_hex = hex::encode(ssz::Encode::as_ssz_bytes(&action));

        assert_eq!(
            is_multisig_update_enacted_in_admin_state(
                &single_signer_admin_state(),
                Authority::SecurityCouncil,
                0,
                &action_hex,
            ),
            Ok(false)
        );
    }

    /// A cancel has no config to compare against, so there is no answer to give — and `Ok(false)`
    /// would be one. The distinction is the module's whole degradation rule: a caller treats
    /// `Ok(false)` as "not enacted" and retires the proposal on it, while an `Err` is logged and
    /// asked again next cycle.
    #[test]
    fn a_cancel_has_no_post_conditions_to_check() {
        let cancel = MultisigAction::Cancel(CancelAction::new(
            0,
            UpdateAction::AlpenAdminMultisig(AlpenAdminMultisigUpdate::new(
                ThresholdConfigUpdate::new(vec![], vec![], NonZeroU8::new(1).unwrap()),
            )),
        ));
        let cancel_hex = hex::encode(ssz::Encode::as_ssz_bytes(&cancel));

        assert!(is_multisig_update_enacted_in_admin_state(
            &single_signer_admin_state(),
            Authority::AlpenAdmin,
            0,
            &cancel_hex,
        )
        .is_err());
    }

    #[tokio::test]
    async fn mock_membership_never_enacted() {
        let action_hex = action_codec::encode_hex(&crate::domain::action::Action::MultisigUpdate(
            crate::domain::action::MultisigUpdate {
                role: Authority::StrataAdmin,
                add_keys: vec![],
                remove_keys: vec![],
                new_threshold: std::num::NonZeroU8::new(2).unwrap(),
            },
        ))
        .unwrap();
        let enacted =
            is_proposal_enacted_on_asm(MOCK_MEMBERSHIP_URL, Authority::StrataAdmin, 1, &action_hex)
                .await
                .unwrap();
        assert!(!enacted);
    }

    #[tokio::test]
    async fn mock_enacted_url_reports_enacted() {
        let action_hex = action_codec::encode_hex(&crate::domain::action::Action::MultisigUpdate(
            crate::domain::action::MultisigUpdate {
                role: Authority::StrataAdmin,
                add_keys: vec![],
                remove_keys: vec![],
                new_threshold: std::num::NonZeroU8::new(2).unwrap(),
            },
        ))
        .unwrap();
        let enacted =
            is_proposal_enacted_on_asm(MOCK_ENACTED_URL, Authority::StrataAdmin, 1, &action_hex)
                .await
                .unwrap();
        assert!(enacted);
    }
}
