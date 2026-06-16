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
    let Some(config_update) = extract_multisig_config_update(&action, authority)? else {
        return Ok(false);
    };
    let role = authority_to_role(authority)?;
    let authority_config = admin
        .authority(role)
        .ok_or_else(|| format!("admin state missing authority for role `{:?}`", role))?;

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

/// Returns `Some(config)` for known multisig-update authority/variant pairs, `None` for
/// non-multisig-update action variants (VkUpdate, etc.), and an error for genuine
/// authority/variant mismatches.
fn extract_multisig_config_update(
    action: &MultisigAction,
    authority: Authority,
) -> Result<Option<&ThresholdConfigUpdate>, String> {
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
            Authority::SecurityCouncil,
            MultisigAction::Update(UpdateAction::StrataSecurityCouncilMultisig(update)),
        ) => Ok(Some(update.config())),
        // MultisigUpdate variant present but wrong authority — data integrity issue.
        (
            _,
            MultisigAction::Update(
                UpdateAction::StrataAdminMultisig(_)
                | UpdateAction::StrataSeqManagerMultisig(_)
                | UpdateAction::StrataSecurityCouncilMultisig(_),
            ),
        ) => {
            Err("action variant does not match proposal authority for enactment check".to_string())
        }
        // Non-multisig-update variants (VkUpdate, etc.) — enactment check not applicable.
        (_, MultisigAction::Update(_)) => Ok(None),
        (_, MultisigAction::Cancel(_)) => {
            Err("cancel actions are not supported for enactment post-condition checks".to_string())
        }
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
    use crate::infrastructure::action_codec;

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
