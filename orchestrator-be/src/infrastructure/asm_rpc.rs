//! One read of the ASM's `strata_asm_getStatus`, decoded into the subprotocol states the
//! orchestrator asks about. Every live ASM read in this crate goes through here.

use serde_json::{json, Value};
use ssz::Decode;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_params::Role;
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};
use strata_asm_proto_bridge_v1::{BridgeV1State, BridgeV1Subproto};
use strata_asm_proto_checkpoint::{CheckpointState, CheckpointSubprotocol};

use crate::error::AppError;
use crate::infrastructure::{http_client, rpc_timeout};

/// The current `AnchorState`, as the ASM reports it.
pub(crate) async fn fetch_anchor_state(rpc_url: &str) -> Result<AnchorState, String> {
    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([])).await?;
    decode_anchor_state_from_status(&status_result)
}

/// The error for a role the admin state does not carry.
pub(crate) fn missing_authority(role: Role) -> AppError {
    AppError::BadRequest(format!("admin state missing authority for role `{role:?}`"))
}

/// The administration subprotocol's state, read live.
pub(crate) async fn fetch_admin_state(
    rpc_url: &str,
) -> Result<AdministrationSubprotoState, String> {
    decode_admin_state(&fetch_anchor_state(rpc_url).await?)
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
        let base = format!("RPC method `{method}` returned JSON-RPC error: {err}");
        if method == "strata_asm_getStatus" && err.to_string().contains("\"code\":-32601") {
            return Err(format!(
				"{base}. This usually means `STRATA_ADMIN_STATE_RPC_URL` points to a non-ASM endpoint."
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

pub(crate) fn decode_admin_state(
    anchor: &AnchorState,
) -> Result<AdministrationSubprotoState, String> {
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
             (see `[database].path` in orchestrator ASM config / asm-config.toml, e.g. /tmp/asm-runner-db) so genesis is recreated."
        )
    })
}

pub(crate) fn decode_bridge_state(anchor: &AnchorState) -> Result<BridgeV1State, String> {
    let id = BridgeV1Subproto::ID;
    let section = anchor.find_section(id).ok_or_else(|| {
        format!("AnchorState has no bridge-v1 subprotocol section (expected id {id})")
    })?;
    section
        .try_to_state::<BridgeV1Subproto>()
        .map_err(|e| format!("BridgeV1 section SSZ decode failed: {e:?}"))
}

pub(crate) fn decode_checkpoint_state(anchor: &AnchorState) -> Result<CheckpointState, String> {
    let id = CheckpointSubprotocol::ID;
    let section = anchor.find_section(id).ok_or_else(|| {
        format!("AnchorState has no checkpoint subprotocol section (expected id {id}).")
    })?;
    section
        .try_to_state::<CheckpointSubprotocol>()
        .map_err(|e| format!("Checkpoint section (id {id}) does not decode with this app ({e:?})."))
}
