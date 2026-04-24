//! JSON-RPC helpers and status decoding for asm-runner E2E tests.

use anyhow::Result;
use reqwest::StatusCode;
use serde_json::{json, Value};
use ssz::Decode;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_params::Role;
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};

pub const RPC_URL: &str = "http://127.0.0.1:8080";

pub async fn rpc_call(method: &str, params: Value) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });

    let response = client.post(RPC_URL).json(&payload).send().await?;
    let status = response.status();
    if status != StatusCode::OK {
        anyhow::bail!("RPC method `{method}` returned unexpected status code: {status}");
    }

    let body: Value = response.json().await?;
    if let Some(err) = body.get("error") {
        anyhow::bail!("RPC method `{method}` returned JSON-RPC error: {err}");
    }

    let result = body.get("result").ok_or_else(|| {
        anyhow::anyhow!("RPC method `{method}` response does not contain `result`: {body}")
    })?;
    Ok(result.clone())
}

fn decode_state_bytes_from_status(status_result: &Value) -> Result<Vec<u8>> {
    let raw_state = status_result
        .pointer("/cur_state/state")
        .or_else(|| status_result.pointer("/current_state/state"))
        .ok_or_else(|| anyhow::anyhow!("status result missing `cur_state.state` array"))?;

    let items = raw_state
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("`cur_state.state` is not an array"))?;

    let bytes = items
        .iter()
        .map(|v| {
            let n = v
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("state entry is not an unsigned integer: {v}"))?;
            u8::try_from(n).map_err(|_| anyhow::anyhow!("state entry out of byte range: {n}"))
        })
        .collect::<anyhow::Result<Vec<u8>>>()?;

    Ok(bytes)
}

fn decode_anchor_state_from_status(status_result: &Value) -> Result<AnchorState> {
    let bytes = decode_state_bytes_from_status(status_result)?;
    AnchorState::from_ssz_bytes(&bytes).map_err(|err| {
        anyhow::anyhow!("failed to SSZ-decode AnchorState from status state bytes: {err}")
    })
}

fn authority_keys_hex(admin: &AdministrationSubprotoState, role: Role) -> Result<Vec<String>> {
    let authority = admin
        .authority(role)
        .ok_or_else(|| anyhow::anyhow!("admin state missing authority for role `{role:?}`"))?;

    Ok(authority
        .config()
        .keys()
        .iter()
        .map(|k| hex::encode(k.serialize()))
        .collect())
}

pub fn assert_expected_admin_keys(
    status_result: &Value,
    expected_admin: &str,
    expected_sequencer: &str,
) -> Result<()> {
    let anchor = decode_anchor_state_from_status(status_result)?;
    let admin = decode_admin_state(&anchor)
        .ok_or_else(|| anyhow::anyhow!("admin state section is missing from AnchorState"))?;

    let admin_keys = authority_keys_hex(&admin, Role::StrataAdministrator)?;
    let sequencer_keys = authority_keys_hex(&admin, Role::StrataSequencerManager)?;

    let has_admin_key = admin_keys
        .iter()
        .any(|key| key.eq_ignore_ascii_case(expected_admin));
    let has_sequencer_key = sequencer_keys
        .iter()
        .any(|key| key.eq_ignore_ascii_case(expected_sequencer));

    if !has_admin_key {
        anyhow::bail!(
            "expected StrataAdministrator key `{expected_admin}` not found in authority keys: {:?}",
            admin_keys
        );
    }

    if !has_sequencer_key {
        anyhow::bail!(
            "expected StrataSequencerManager key `{expected_sequencer}` not found in authority keys: {:?}",
            sequencer_keys
        );
    }

    Ok(())
}

fn decode_admin_state(anchor: &AnchorState) -> Option<AdministrationSubprotoState> {
    anchor
        .find_section(AdministrationSubprotocol::ID)
        .and_then(|section| section.try_to_state::<AdministrationSubprotocol>().ok())
}
