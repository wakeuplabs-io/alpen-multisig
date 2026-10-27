use crate::domain::auth::AuthRole;
use crate::domain::authority::Authority;

/// Return the ordered list of Admin IDs for an authority's signer set.
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
        // G and 2·G, as regtest Admin IDs.
        Authority::StrataAdmin | Authority::SequencerManager | Authority::SecurityCouncil => {
            Some(vec![
                "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080".to_string(),
                "bcrt1qq6hag67dl53wl99vzg42z8eyzfz2xlkvwk6f7m".to_string(),
            ])
        }
        _ => None,
    }
}

#[cfg(not(any(test, feature = "dev-mocks")))]
fn mock_ordered_keys(_rpc_url: &str, _authority: Authority) -> Option<Vec<String>> {
    None
}
