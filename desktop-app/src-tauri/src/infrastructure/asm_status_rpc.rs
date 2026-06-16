use std::collections::HashMap;

use serde_json::{json, Value};
use ssz::Decode;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};
use strata_asm_proto_bridge_v1::{BridgeV1State, BridgeV1Subproto};
use strata_asm_proto_checkpoint::{CheckpointState, CheckpointSubprotocol};
use strata_asm_txs_admin::actions::MultisigAction;

use crate::domain::auth::AuthRole;

pub fn default_rpc_url() -> String {
    "http://127.0.0.1:8080".to_string()
}

pub struct MultisigConfig {
    pub signers: Vec<String>,
    pub threshold: u8,
}

pub struct CurrentVk {
    pub type_id: u8,
    pub type_name: String,
    pub condition_hex: String,
}

pub async fn fetch_multisig_config(
    rpc_url: &str,
    role: AuthRole,
) -> Result<MultisigConfig, String> {
    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([])).await?;
    let anchor = decode_anchor_state_from_status(&status_result)?;
    let admin = decode_admin_state(&anchor)?;

    let authority = admin
        .authority(role.to_upstream_role())
        .ok_or_else(|| format!("admin state missing authority for role `{role:?}`"))?;

    Ok(MultisigConfig {
        signers: authority
            .config()
            .keys()
            .iter()
            .map(|k| hex::encode(k.serialize()))
            .collect(),
        threshold: authority.config().threshold(),
    })
}

pub async fn fetch_role_membership(
    rpc_url: &str,
) -> Result<(HashMap<AuthRole, Vec<String>>, u64), String> {
    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([])).await?;
    let anchor = decode_anchor_state_from_status(&status_result)?;
    let admin = decode_admin_state(&anchor)?;

    let mut role_to_keys = HashMap::new();
    role_to_keys.insert(
        AuthRole::StrataAdministrator,
        authority_keys_hex(&admin, AuthRole::StrataAdministrator)?,
    );
    role_to_keys.insert(
        AuthRole::StrataSequencerManager,
        authority_keys_hex(&admin, AuthRole::StrataSequencerManager)?,
    );
    role_to_keys.insert(
        AuthRole::AlpenAdministrator,
        authority_keys_hex(&admin, AuthRole::AlpenAdministrator)?,
    );
    role_to_keys.insert(
        AuthRole::StrataSecurityCouncil,
        authority_keys_hex(&admin, AuthRole::StrataSecurityCouncil)?,
    );

    Ok((role_to_keys, now_unix_ms()))
}

pub async fn fetch_current_vk(rpc_url: &str) -> Result<CurrentVk, String> {
    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([])).await?;
    let anchor = decode_anchor_state_from_status(&status_result)?;
    let checkpoint = decode_checkpoint_state(&anchor)?;
    let predicate = checkpoint.checkpoint_predicate();
    let type_name = match predicate.id() {
        0 => "NeverAccept",
        1 => "AlwaysAccept",
        10 => "Bip340Schnorr",
        20 => "Sp1Groth16",
        _ => "Unknown",
    }
    .to_string();
    Ok(CurrentVk {
        type_id: predicate.id(),
        type_name,
        condition_hex: hex::encode(predicate.condition()),
    })
}

pub async fn fetch_current_operators(rpc_url: &str) -> Result<Vec<String>, String> {
    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([])).await?;
    let anchor = decode_anchor_state_from_status(&status_result)?;
    let bridge = decode_bridge_state(&anchor)?;
    Ok(bridge
        .operators()
        .operators()
        .iter()
        .map(|entry| hex::encode(entry.musig2_pk().x_only_public_key().0.serialize()))
        .collect())
}

/// Search the live ASM queue for the `UpdateId` matching `action_hex`.
///
/// Returns `None` when the update is not yet queued (reveal not processed by ASM yet) or
/// when `action_hex` encodes a Cancel (not an Update).
pub async fn find_update_id_in_queue(
    rpc_url: &str,
    action_hex: &str,
) -> Result<Option<u32>, String> {
    let bytes = hex::decode(action_hex.trim_start_matches("0x"))
        .map_err(|e| format!("invalid action hex: {e}"))?;
    let target_update = match MultisigAction::from_ssz_bytes(&bytes)
        .map_err(|e| format!("invalid SSZ MultisigAction: {e:?}"))?
    {
        MultisigAction::Update(u) => u,
        MultisigAction::Cancel(_) => return Ok(None),
    };

    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([])).await?;
    let anchor = decode_anchor_state_from_status(&status_result)?;
    let admin = decode_admin_state(&anchor)?;

    let found = admin
        .queued()
        .iter()
        .find(|q| q.action() == &target_update)
        .map(|q| *q.id());

    Ok(found)
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
        let base = format!("RPC method `{method}` returned JSON-RPC error: {err}");
        if method == "strata_asm_getStatus" && err.to_string().contains("\"code\":-32601") {
            return Err(format!(
                "{base}. This usually means the Strata RPC URL in Node Configuration points to a non-ASM endpoint (for example the public Alpen RPC). Set it to an ASM runner RPC URL."
            ));
        }
        return Err(base);
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

fn decode_checkpoint_state(anchor: &AnchorState) -> Result<CheckpointState, String> {
    let id = CheckpointSubprotocol::ID;
    let section = anchor.find_section(id).ok_or_else(|| {
        format!("AnchorState has no checkpoint subprotocol section (expected id {id})")
    })?;
    section
        .try_to_state::<CheckpointSubprotocol>()
        .map_err(|e| format!("Checkpoint section SSZ decode failed: {e:?}"))
}

fn decode_admin_state(anchor: &AnchorState) -> Result<AdministrationSubprotoState, String> {
    let id = AdministrationSubprotocol::ID;
    let section = anchor.find_section(id).ok_or_else(|| {
        format!(
            "AnchorState has no administration subprotocol section (expected id {id}). \
             The RPC returned a decodable `AnchorState`, but it does not include admin — \
             often wrong `strata-asm-runner` spec/params, or state from an incompatible DB snapshot."
        )
    })?;
    section.try_to_state::<AdministrationSubprotocol>().map_err(|e| {
        format!(
            "Administration section (id {id}) is present but its SSZ payload does not decode with this app ({e:?}). \
             Rebuild `strata-asm-runner` from the same `alpenlabs/asm` commit as this workspace and delete the runner DB \
             (see `[database].path` in asm-config.toml, e.g. /tmp/asm-runner-db) so genesis is recreated."
        )
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

fn authority_keys_hex(
    admin: &AdministrationSubprotoState,
    role: AuthRole,
) -> Result<Vec<String>, String> {
    let authority = admin
        .authority(role.to_upstream_role())
        .ok_or_else(|| format!("admin state missing authority for role `{role:?}`"))?;

    Ok(authority
        .config()
        .keys()
        .iter()
        .map(|k| hex::encode(k.serialize()))
        .collect())
}

fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_state_bytes_supports_cur_state() {
        let status = json!({
            "cur_state": {
                "state": [0, 1, 2, 255]
            }
        });
        let bytes = decode_state_bytes_from_status(&status).expect("should decode bytes");
        assert_eq!(bytes, vec![0, 1, 2, 255]);
    }

    #[test]
    fn decode_state_bytes_supports_current_state() {
        let status = json!({
            "current_state": {
                "state": [5, 6]
            }
        });
        let bytes = decode_state_bytes_from_status(&status).expect("should decode bytes");
        assert_eq!(bytes, vec![5, 6]);
    }

    #[test]
    fn decode_state_bytes_rejects_out_of_range_items() {
        let status = json!({
            "cur_state": {
                "state": [300]
            }
        });
        let err = decode_state_bytes_from_status(&status).unwrap_err();
        assert!(err.contains("out of byte range"));
    }
}
