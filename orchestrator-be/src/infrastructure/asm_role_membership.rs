use std::collections::HashMap;

use serde_json::{json, Value};
use ssz::Decode;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_params::Role;
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};

use crate::domain::authority::Authority;
use crate::error::AppError;

pub(crate) async fn is_signer_member_for_authority(
    rpc_url: &str,
    authority: Authority,
    signer_pubkey: &str,
) -> Result<bool, AppError> {
    #[cfg(test)]
    if let Some(is_member) = mock_membership_for_tests(rpc_url, authority, signer_pubkey) {
        return Ok(is_member);
    }

    let role_membership = fetch_role_membership(rpc_url)
        .await
        .map_err(AppError::BadRequest)?;
    let role = authority_to_role(authority).map_err(AppError::BadRequest)?;
    let keys = role_membership.get(&role).ok_or_else(|| {
        AppError::BadRequest(format!(
            "admin state missing authority for role `{:?}`",
            role
        ))
    })?;
    Ok(keys
        .iter()
        .any(|key| key.eq_ignore_ascii_case(signer_pubkey)))
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

async fn fetch_role_membership(rpc_url: &str) -> Result<HashMap<Role, Vec<String>>, String> {
    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([])).await?;
    let anchor = decode_anchor_state_from_status(&status_result)?;
    let admin = decode_admin_state(&anchor)
        .ok_or_else(|| "admin state section is missing from AnchorState".to_string())?;

    let mut role_to_keys = HashMap::new();
    role_to_keys.insert(
        Role::StrataAdministrator,
        authority_keys_hex(&admin, Role::StrataAdministrator)?,
    );
    role_to_keys.insert(
        Role::StrataSequencerManager,
        authority_keys_hex(&admin, Role::StrataSequencerManager)?,
    );

    Ok(role_to_keys)
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

fn decode_admin_state(anchor: &AnchorState) -> Option<AdministrationSubprotoState> {
    anchor
        .find_section(AdministrationSubprotocol::ID)
        .and_then(|section| section.try_to_state::<AdministrationSubprotocol>().ok())
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

#[cfg(test)]
fn mock_membership_for_tests(
    rpc_url: &str,
    authority: Authority,
    signer_pubkey: &str,
) -> Option<bool> {
    if rpc_url != "mock://asm-membership" {
        return None;
    }

    let is_member = match authority {
        Authority::StrataAdmin => {
            signer_pubkey.eq_ignore_ascii_case(
                "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
            ) || signer_pubkey.eq_ignore_ascii_case(
                "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5",
            )
        }
        Authority::SequencerManager => false,
        _ => return None,
    };
    Some(is_member)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_mapping_is_fail_closed_for_unmapped_authorities() {
        assert!(authority_to_role(Authority::StrataAdmin).is_ok());
        assert!(authority_to_role(Authority::SequencerManager).is_ok());
        assert!(authority_to_role(Authority::AlpenAdmin).is_err());
        assert!(authority_to_role(Authority::SecurityCouncil).is_err());
        assert!(authority_to_role(Authority::PayoutAdmin).is_err());
    }

    #[test]
    fn mock_membership_matches_signers_case_insensitive() {
        let is_member = mock_membership_for_tests(
            "mock://asm-membership",
            Authority::StrataAdmin,
            "0279BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798",
        );
        assert_eq!(is_member, Some(true));
    }
}
