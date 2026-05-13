use std::collections::HashMap;

use serde_json::{json, Value};
use ssz::Decode;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};

use crate::domain::auth::AuthRole;

const RPC_URL_ENV_VAR: &str = "STRATA_ADMIN_STATE_RPC_URL";
const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8080";

pub fn default_rpc_url() -> String {
    std::env::var(RPC_URL_ENV_VAR)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_RPC_URL.to_string())
}

pub struct MultisigConfig {
    pub signers: Vec<String>,
    pub threshold: u8,
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

    Ok((role_to_keys, now_unix_ms()))
}

async fn rpc_call(rpc_url: &str, method: &str, params: Value) -> Result<Value, String> {
    let client = reqwest::Client::new();
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
                "{base}. This usually means `STRATA_ADMIN_STATE_RPC_URL` points to a non-ASM endpoint (for example the public Alpen RPC). Set it to an ASM runner RPC URL."
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
