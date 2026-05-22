use serde_json::{json, Value};
use ssz::Decode;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_params::Role;
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};

use crate::domain::authority::Authority;

/// Return the ordered list of hex-encoded compressed public keys for an authority's signer set.
///
/// The order is canonical (as stored in ASM state) and determines the signer index used in
/// `IndexedSignature`. Called during broadcast to map stored pubkeys to their indices.
pub async fn ordered_keys_for_authority(
    rpc_url: &str,
    authority: Authority,
) -> Result<Vec<String>, String> {
    if let Some(keys) = mock_ordered_keys(rpc_url, authority) {
        return Ok(keys);
    }
    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([])).await?;
    let anchor = decode_anchor_state_from_status(&status_result)?;
    let admin = decode_admin_state(&anchor)?;
    let role = authority_to_role(authority)?;
    authority_keys_hex(&admin, role)
}

fn authority_to_role(authority: Authority) -> Result<Role, String> {
    match authority {
        Authority::StrataAdmin => Ok(Role::StrataAdministrator),
        Authority::SequencerManager => Ok(Role::StrataSequencerManager),
        _ => Err(format!(
            "authority `{authority:?}` is not mapped to ASM role authorization yet"
        )),
    }
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
                "{base}. This usually means the ASM RPC URL points to a non-ASM endpoint."
            ));
        }
        return Err(base);
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

fn authority_keys_hex(
    admin: &AdministrationSubprotoState,
    role: Role,
) -> Result<Vec<String>, String> {
    let authority = admin
        .authority(role)
        .ok_or_else(|| format!("admin state missing authority for role `{:?}`", role))?;

    Ok(authority
        .config()
        .keys()
        .iter()
        .map(|k| hex::encode(k.serialize()))
        .collect())
}

fn mock_ordered_keys(rpc_url: &str, authority: Authority) -> Option<Vec<String>> {
    if rpc_url != "mock://asm-membership" {
        return None;
    }
    match authority {
        Authority::StrataAdmin => Some(vec![
            "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798".to_string(),
            "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5".to_string(),
        ]),
        Authority::SequencerManager => Some(vec![
            "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798".to_string(),
            "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5".to_string(),
        ]),
        _ => None,
    }
}
