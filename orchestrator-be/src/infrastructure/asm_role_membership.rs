use std::collections::HashMap;

use serde_json::{json, Value};
use ssz::Decode;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_params::Role;
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};

use crate::domain::authority::Authority;
use crate::error::AppError;

/// Whether this authority has a wired ASM `Role` mapping (P-037).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthorityAsmSupport {
    Supported,
    Unsupported,
}

pub(crate) fn authority_asm_support(authority: Authority) -> AuthorityAsmSupport {
    match authority_to_role(authority) {
        Ok(_) => AuthorityAsmSupport::Supported,
        Err(_) => AuthorityAsmSupport::Unsupported,
    }
}

pub(crate) async fn is_signer_member_for_authority(
    rpc_url: &str,
    authority: Authority,
    signer_pubkey: &str,
) -> Result<bool, AppError> {
    #[cfg(test)]
    if let Some(is_member) = test_mocks::mock_membership(rpc_url, authority, signer_pubkey) {
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

// TODO: decouple mock from implementation
pub(crate) async fn last_seqno_for_authority(
    rpc_url: &str,
    authority: Authority,
) -> Result<u64, AppError> {
    #[cfg(test)]
    if let Some(seqno) = test_mocks::mock_last_seqno(rpc_url, authority) {
        return Ok(seqno);
    }

    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([]))
        .await
        .map_err(AppError::BadRequest)?;
    let anchor = decode_anchor_state_from_status(&status_result).map_err(AppError::BadRequest)?;
    let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;
    let role = authority_to_role(authority).map_err(AppError::BadRequest)?;
    let authority_config = admin.authority(role).ok_or_else(|| {
        AppError::BadRequest(format!(
            "admin state missing authority for role `{:?}`",
            role
        ))
    })?;
    Ok(authority_config.last_seqno())
}

pub(crate) async fn threshold_for_authority(
    rpc_url: &str,
    authority: Authority,
) -> Result<u16, AppError> {
    #[cfg(test)]
    if let Some(threshold) = test_mocks::mock_threshold(rpc_url, authority) {
        return Ok(threshold);
    }

    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([]))
        .await
        .map_err(AppError::BadRequest)?;
    let anchor = decode_anchor_state_from_status(&status_result).map_err(AppError::BadRequest)?;
    let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;
    let role = authority_to_role(authority).map_err(AppError::BadRequest)?;
    let authority_config = admin.authority(role).ok_or_else(|| {
        AppError::BadRequest(format!(
            "admin state missing authority for role `{:?}`",
            role
        ))
    })?;
    Ok(u16::from(authority_config.config().threshold()))
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
    let admin = decode_admin_state(&anchor)?;

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
             (see `[database].path` in orchestrator ASM config / asm-config.toml, e.g. /tmp/asm-runner-db) so genesis is recreated."
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

#[cfg(test)]
mod test_mocks {
    use super::*;

    pub(super) fn mock_membership(
        rpc_url: &str,
        authority: Authority,
        signer_pubkey: &str,
    ) -> Option<bool> {
        mock_membership_impl(rpc_url, authority, signer_pubkey)
    }

    pub(super) fn mock_last_seqno(rpc_url: &str, authority: Authority) -> Option<u64> {
        mock_last_seqno_impl(rpc_url, authority)
    }

    pub(super) fn mock_threshold(rpc_url: &str, authority: Authority) -> Option<u16> {
        mock_threshold_impl(rpc_url, authority)
    }

    fn mock_strata_signer_b_pk_matches(signer_pubkey: &str) -> bool {
        use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
        let mut sk_bytes = [0u8; 32];
        sk_bytes[31] = 2;
        let Ok(sk) = SecretKey::from_slice(&sk_bytes) else {
            return false;
        };
        let pk = PublicKey::from_secret_key(&Secp256k1::new(), &sk);
        signer_pubkey.eq_ignore_ascii_case(&hex::encode(pk.serialize()))
    }

    fn mock_membership_impl(rpc_url: &str, authority: Authority, signer_pubkey: &str) -> Option<bool> {
    if rpc_url != "mock://asm-membership" {
        return None;
    }

    let is_member = match authority {
        Authority::StrataAdmin => {
            signer_pubkey.eq_ignore_ascii_case(
                "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
            ) || mock_strata_signer_b_pk_matches(signer_pubkey)
        }
        Authority::SequencerManager => false,
        _ => return None,
    };
        Some(is_member)
    }

    fn mock_last_seqno_impl(rpc_url: &str, authority: Authority) -> Option<u64> {
        if rpc_url != "mock://asm-membership" {
            return None;
        }
        match authority {
            Authority::StrataAdmin => Some(0),
            Authority::SequencerManager => Some(0),
            _ => None,
        }
    }

    fn mock_threshold_impl(rpc_url: &str, authority: Authority) -> Option<u16> {
        if rpc_url != "mock://asm-membership" {
            return None;
        }

        match authority {
            Authority::StrataAdmin => Some(2),
            Authority::SequencerManager => Some(2),
            _ => None,
        }
    }
}

#[cfg(test)]
fn mock_membership(rpc_url: &str, authority: Authority, signer_pubkey: &str) -> Option<bool> {
    test_mocks::mock_membership(rpc_url, authority, signer_pubkey)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_five_authorities_have_explicit_asm_mapping_status() {
        use Authority::*;
        assert_eq!(
            authority_asm_support(StrataAdmin),
            AuthorityAsmSupport::Supported
        );
        assert_eq!(
            authority_asm_support(SequencerManager),
            AuthorityAsmSupport::Supported
        );
        assert_eq!(
            authority_asm_support(AlpenAdmin),
            AuthorityAsmSupport::Unsupported
        );
        assert_eq!(
            authority_asm_support(SecurityCouncil),
            AuthorityAsmSupport::Unsupported
        );
        assert_eq!(
            authority_asm_support(PayoutAdmin),
            AuthorityAsmSupport::Unsupported
        );
    }

    #[test]
    fn mock_membership_matches_signers_case_insensitive() {
        let is_member = mock_membership(
            "mock://asm-membership",
            Authority::StrataAdmin,
            "0279BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798",
        );
        assert_eq!(is_member, Some(true));
    }
}
