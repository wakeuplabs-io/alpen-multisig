use crate::domain::auth::AuthRole;
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
    super::reject_mock_asm_url_in_prod(rpc_url)?;
    let role = AuthRole::try_for_authority(authority)?;
    Ok(super::asm_status_rpc::fetch_multisig_config(rpc_url, role)
        .await?
        .signers)
}

// In-process ASM signer-set mock — compiled only under `cfg(test)` or `dev-mocks`.
// In production builds this is an inert stub returning `None`.
#[cfg(any(test, feature = "dev-mocks"))]
fn mock_ordered_keys(rpc_url: &str, authority: Authority) -> Option<Vec<String>> {
    if rpc_url != "mock://asm-membership" {
        return None;
    }
    match authority {
        // One signer pair for all three: the local stack authenticates every role with one wallet.
        Authority::StrataAdmin | Authority::SequencerManager | Authority::SecurityCouncil => {
            Some(vec![
                "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798".to_string(),
                "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5".to_string(),
            ])
        }
        _ => None,
    }
}

#[cfg(not(any(test, feature = "dev-mocks")))]
fn mock_ordered_keys(_rpc_url: &str, _authority: Authority) -> Option<Vec<String>> {
    None
}
