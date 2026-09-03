use std::collections::HashMap;

use serde_json::{json, Value};
use ssz::Decode;
use strata_asm_common::{AnchorState, Subprotocol};
use strata_asm_params::{Role, UpdateTxType};
use strata_asm_proto_administration::{AdministrationSubprotoState, AdministrationSubprotocol};
use strata_asm_txs_admin::actions::MultisigAction;

use crate::domain::authority::Authority;
use crate::error::AppError;
use crate::infrastructure::{action_codec, rpc_timeout};

/// Whether this authority has a wired ASM `Role` mapping (P-037).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthorityAsmSupport {
    Supported,
    Unsupported,
}

pub(crate) fn authority_asm_support(authority: Authority) -> AuthorityAsmSupport {
    match authority_to_role_impl(authority) {
        Ok(_) => AuthorityAsmSupport::Supported,
        Err(_) => AuthorityAsmSupport::Unsupported,
    }
}

pub(crate) async fn is_signer_member_for_authority(
    rpc_url: &str,
    authority: Authority,
    signer_pubkey: &str,
) -> Result<bool, AppError> {
    if let Some(is_member) = mock_membership(rpc_url, authority, signer_pubkey) {
        return Ok(is_member);
    }

    if authority_asm_support(authority) == AuthorityAsmSupport::Unsupported {
        return Err(AppError::BadRequest(format!(
            "authority `{authority:?}` is not mapped to ASM role authorization yet"
        )));
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
    if let Some(seqno) = mock_last_seqno(rpc_url, authority) {
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
    if let Some(threshold) = mock_threshold(rpc_url, authority) {
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

/// Return the confirmation depth (in blocks) before the update in `action_hex` activates.
///
/// Resolved from the action, never from the authority: the Security Council signs both Defcon 1
/// (immediate) and Defcon 3 (timelocked), so no per-authority mapping can answer for it. Read live
/// on every call — see docs/specs/security-council-defcon-phase-1.md.
///
/// Returns `0` for actions that bypass the queue and apply immediately.
pub(crate) async fn lock_period_for_action(
    rpc_url: &str,
    action_hex: &str,
) -> Result<u64, AppError> {
    let action =
        action_codec::decode_multisig_action_hex(action_hex).map_err(AppError::BadRequest)?;

    if let Some(period) = mock_lock_period(rpc_url, &action) {
        return Ok(period);
    }

    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([]))
        .await
        .map_err(AppError::BadRequest)?;
    let anchor = decode_anchor_state_from_status(&status_result).map_err(AppError::BadRequest)?;
    let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;

    Ok(depth_for_action(&action, |tx_type| {
        admin.confirmation_depth(tx_type)
    }))
}

/// Resolve `action`'s confirmation depth through `depth_of`, upstream's per-tx-type lookup.
///
/// The lookup is a parameter so the decision is testable without an ASM. Upstream owns the table
/// (`UpdateAction::update_tx_type` and `ConfirmationDepths::get`); this only dispatches into it.
///
/// A cancel resolves to `0`: it is never enqueued, it applies when it confirms. Deliberately not
/// the depth of the update it targets, which would be a plausible-looking wrong number.
fn depth_for_action(
    action: &MultisigAction,
    depth_of: impl Fn(UpdateTxType) -> Option<u16>,
) -> u64 {
    match action {
        MultisigAction::Update(update) => u64::from(depth_of(update.update_tx_type()).unwrap_or(0)),
        MultisigAction::Cancel(_) => 0,
    }
}

/// Live confirmation depths for one HTTP request — one `strata_asm_getStatus` per fetch.
pub(crate) enum ConfirmationDepthResolver {
    Live(AdministrationSubprotoState),
    #[cfg(any(test, feature = "dev-mocks"))]
    Mock(strata_asm_params::ConfirmationDepths),
    Unavailable,
}

impl ConfirmationDepthResolver {
    pub async fn fetch(rpc_url: &str) -> Self {
        #[cfg(any(test, feature = "dev-mocks"))]
        if rpc_url == "mock://asm-membership" {
            return Self::Mock(uniform_confirmation_depths(2016));
        }

        match fetch_admin_state(rpc_url).await {
            Ok(admin) => Self::Live(admin),
            Err(e) => {
                tracing::warn!("cancelability: confirmation depth lookup failed: {e}");
                Self::Unavailable
            }
        }
    }

    fn depth(&self, tx_type: UpdateTxType) -> Option<u16> {
        match self {
            Self::Live(admin) => admin.confirmation_depth(tx_type),
            #[cfg(any(test, feature = "dev-mocks"))]
            Self::Mock(depths) => depths.get(tx_type),
            Self::Unavailable => None,
        }
    }

    /// Whether the action can be cancelled on chain — the same gate `create_cancel_proposal`
    /// applies: a non-zero confirmation depth means the update is enqueued and a cancel can reach
    /// it. An action nobody can decode, and an ASM nobody could reach, both answer "no affordance".
    pub fn is_cancelable_for_hex(&self, action_hex: &str) -> bool {
        let Ok(action) = action_codec::decode_multisig_action_hex(action_hex) else {
            return false;
        };
        depth_for_action(&action, |tx_type| self.depth(tx_type)) > 0
    }
}

async fn fetch_admin_state(rpc_url: &str) -> Result<AdministrationSubprotoState, String> {
    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([])).await?;
    let anchor = decode_anchor_state_from_status(&status_result)?;
    decode_admin_state(&anchor)
}

/// Find the ASM queue `UpdateId` for the update encoded in `action_hex`.
///
/// Decodes the action, then scans the live ASM queue for the matching `UpdateAction`.
/// Returns `None` when the update is not yet in the queue (reveal not confirmed) or
/// the RPC URL is a mock endpoint (tests).
pub(crate) async fn update_id_in_queue_for_action(
    rpc_url: &str,
    action_hex: &str,
) -> Result<Option<u32>, AppError> {
    if is_mock_url(rpc_url) {
        return Ok(None);
    }

    let action =
        action_codec::decode_multisig_action_hex(action_hex).map_err(AppError::BadRequest)?;
    let target_update = match action {
        MultisigAction::Update(u) => u,
        MultisigAction::Cancel(_) => return Ok(None),
    };

    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([]))
        .await
        .map_err(AppError::BadRequest)?;
    let anchor = decode_anchor_state_from_status(&status_result).map_err(AppError::BadRequest)?;
    let admin = decode_admin_state(&anchor).map_err(AppError::BadRequest)?;

    let found = admin
        .queued()
        .iter()
        .find(|q| q.action() == &target_update)
        .map(|q| *q.id());

    Ok(found)
}

/// Refuse an action the session's authority is not allowed to sign (AC 17).
///
/// The role that may sign an update is upstream's table (`UpdateTxType::authorized_role`), not a
/// copy of ours, so a new update variant is gated correctly here the moment it exists. Reads no
/// chain state — hence sync, unlike its neighbours in this module.
///
/// See docs/specs/security-council-defcon-phase-3.md §5.
pub(crate) fn require_authorized_for_action(
    authority: Authority,
    action: &MultisigAction,
) -> Result<(), AppError> {
    // A cancel carries no `UpdateTxType` and so no authorized role. Cancels are created through
    // their own endpoint, gated on the target's confirmation depth (Phase 2).
    let MultisigAction::Update(update) = action else {
        return Ok(());
    };

    let tx_type = update.update_tx_type();
    let required = tx_type.authorized_role();
    let session_role = authority_to_role(authority).map_err(AppError::BadRequest)?;

    if session_role != required {
        return Err(AppError::BadRequest(format!(
            "action `{}` must be authorized by `{required}`, but the session is `{session_role}`",
            tx_type.name()
        )));
    }
    Ok(())
}

fn authority_to_role(authority: Authority) -> Result<Role, String> {
    authority_to_role_impl(authority)
}

fn authority_to_role_impl(authority: Authority) -> Result<Role, String> {
    match authority {
        Authority::StrataAdmin => Ok(Role::StrataAdministrator),
        Authority::SequencerManager => Ok(Role::StrataSequencerManager),
        Authority::AlpenAdmin => Ok(Role::AlpenAdministrator),
        Authority::SecurityCouncil => Ok(Role::StrataSecurityCouncil),
        _ => Err(format!(
            "authority `{authority:?}` is not mapped to ASM role authorization yet"
        )),
    }
}

async fn fetch_role_membership(rpc_url: &str) -> Result<HashMap<Role, Vec<String>>, String> {
    let status_result = rpc_call(rpc_url, "strata_asm_getStatus", json!([])).await?;
    let anchor = decode_anchor_state_from_status(&status_result)?;
    let admin = decode_admin_state(&anchor)?;

    // A role the chain does not carry is "not a member", never "membership unknowable". This is
    // read for every authority on every auth challenge, so one authority missing from an older
    // genesis must not refuse the login of the three that are there.
    let mut role_to_keys = HashMap::new();
    for role in [
        Role::StrataAdministrator,
        Role::StrataSequencerManager,
        Role::AlpenAdministrator,
        Role::StrataSecurityCouncil,
    ] {
        match authority_keys_hex(&admin, role) {
            Ok(keys) => {
                role_to_keys.insert(role, keys);
            }
            Err(e) => {
                tracing::warn!(role = ?role, error = %e, "skipping authority absent from admin state")
            }
        }
    }

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

// ─── In-process ASM mock (dev / e2e only) ────────────────────────────────────
//
// Everything below is compiled ONLY under `cfg(test)` or the `dev-mocks` feature.
// In production builds these become inert stubs (`None` / `false`), so a `mock://`
// RPC URL can never satisfy an authorization check — it falls through to the real
// RPC path (and is additionally rejected at startup by `Config::from_env`).

#[cfg(any(test, feature = "dev-mocks"))]
fn is_mock_url(rpc_url: &str) -> bool {
    rpc_url == "mock://asm-membership"
        || rpc_url == crate::infrastructure::asm_enactment::MOCK_ENACTED_URL
        || rpc_url == crate::infrastructure::asm_enactment::MOCK_ENACTED_AHEAD_URL
        || rpc_url == crate::infrastructure::asm_enactment::MOCK_SEQNO_AHEAD_URL
}

#[cfg(not(any(test, feature = "dev-mocks")))]
fn is_mock_url(_rpc_url: &str) -> bool {
    false
}

#[cfg(any(test, feature = "dev-mocks"))]
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

/// In-process mock for e2e and local dev when `STRATA_ADMIN_STATE_RPC_URL=mock://asm-membership`.
#[cfg(any(test, feature = "dev-mocks"))]
fn mock_membership(rpc_url: &str, authority: Authority, signer_pubkey: &str) -> Option<bool> {
    if rpc_url != "mock://asm-membership" {
        return None;
    }

    let is_member = match authority {
        Authority::StrataAdmin => {
            signer_pubkey.eq_ignore_ascii_case(
                "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
            ) || mock_strata_signer_b_pk_matches(signer_pubkey)
        }
        Authority::AlpenAdmin => signer_pubkey.eq_ignore_ascii_case(
            "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
        ),
        // Same signer pair as the Strata admin: the local stack authenticates the two
        // roles with one wallet, and a council-only key would only add a second mnemonic
        // to every manual test of the Defcon flow.
        Authority::SecurityCouncil => {
            signer_pubkey.eq_ignore_ascii_case(
                "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
            ) || mock_strata_signer_b_pk_matches(signer_pubkey)
        }
        Authority::SequencerManager => false,
        _ => return None,
    };
    Some(is_member)
}

#[cfg(not(any(test, feature = "dev-mocks")))]
fn mock_membership(_rpc_url: &str, _authority: Authority, _signer_pubkey: &str) -> Option<bool> {
    None
}

#[cfg(any(test, feature = "dev-mocks"))]
fn mock_last_seqno(rpc_url: &str, authority: Authority) -> Option<u64> {
    // Two chains that have moved past the proposals under test: one enacts them, one does not.
    if rpc_url == crate::infrastructure::asm_enactment::MOCK_SEQNO_AHEAD_URL
        || rpc_url == crate::infrastructure::asm_enactment::MOCK_ENACTED_AHEAD_URL
    {
        return Some(5);
    }
    if rpc_url != "mock://asm-membership" {
        return None;
    }
    match authority {
        Authority::StrataAdmin => Some(0),
        Authority::SequencerManager => Some(0),
        Authority::AlpenAdmin => Some(0),
        Authority::SecurityCouncil => Some(0),
        _ => None,
    }
}

#[cfg(not(any(test, feature = "dev-mocks")))]
fn mock_last_seqno(_rpc_url: &str, _authority: Authority) -> Option<u64> {
    None
}

#[cfg(any(test, feature = "dev-mocks"))]
fn mock_threshold(rpc_url: &str, authority: Authority) -> Option<u16> {
    // The seqno fixtures answer the same threshold as the membership one: they exist to move
    // `last_seqno`, not to drift the snapshot.
    let known = rpc_url == "mock://asm-membership"
        || rpc_url == crate::infrastructure::asm_enactment::MOCK_SEQNO_AHEAD_URL
        || rpc_url == crate::infrastructure::asm_enactment::MOCK_ENACTED_AHEAD_URL;
    if !known {
        return None;
    }

    match authority {
        Authority::StrataAdmin => Some(2),
        Authority::SequencerManager => Some(2),
        Authority::AlpenAdmin => Some(2),
        Authority::SecurityCouncil => Some(2),
        _ => None,
    }
}

#[cfg(not(any(test, feature = "dev-mocks")))]
fn mock_threshold(_rpc_url: &str, _authority: Authority) -> Option<u16> {
    None
}

/// Every depth set to `depth`. Upstream's `get` still overrides the variants it hardcodes, which is
/// the point: a fixture cannot give Defcon 1 a lock period.
#[cfg(any(test, feature = "dev-mocks"))]
fn uniform_confirmation_depths(depth: u16) -> strata_asm_params::ConfirmationDepths {
    strata_asm_params::ConfirmationDepths {
        strata_admin_multisig_update: depth,
        strata_seq_manager_multisig_update: depth,
        alpen_admin_multisig_update: depth,
        strata_security_council_multisig_update: depth,
        operator_update: depth,
        sequencer_update: depth,
        ol_stf_vk_update: depth,
        asm_stf_vk_update: depth,
        ee_stf_vk_update: depth,
        defcon3: depth,
        safe_harbour_address_update: depth,
    }
}

/// Uniform 2016-block depths, dispatched through `depth_for_action` rather than answered directly.
///
/// Going through the real dispatch is what keeps the mock honest: upstream forces Defcon 1 to `0`
/// whatever the fixture says, so the dev stack cannot show a lock period for an action that applies
/// immediately.
#[cfg(any(test, feature = "dev-mocks"))]
fn mock_lock_period(rpc_url: &str, action: &MultisigAction) -> Option<u64> {
    if rpc_url != "mock://asm-membership" {
        return None;
    }

    let depths = uniform_confirmation_depths(2016);
    Some(depth_for_action(action, |tx_type| depths.get(tx_type)))
}

#[cfg(not(any(test, feature = "dev-mocks")))]
fn mock_lock_period(_rpc_url: &str, _action: &MultisigAction) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use strata_asm_txs_admin::actions::updates::{
        Defcon1Update, Defcon3Update, OperatorSetUpdate, StrataAdminMultisigUpdate,
    };
    use strata_asm_txs_admin::actions::{CancelAction, UpdateAction};
    use strata_crypto::threshold_signature::ThresholdConfigUpdate;

    use super::*;

    /// Fixtures start from a non-zero depth for every variant, so a `0` in an assertion can only
    /// come from upstream's hardcoded arm and never from an unset field.
    const NON_ZERO_BASELINE: u16 = 1;

    fn signer_update() -> MultisigAction {
        let config_update =
            ThresholdConfigUpdate::new(vec![], vec![], NonZeroU8::new(2).expect("threshold"));
        MultisigAction::Update(UpdateAction::StrataAdminMultisig(
            StrataAdminMultisigUpdate::new(config_update),
        ))
    }

    fn operator_set_update() -> MultisigAction {
        MultisigAction::Update(UpdateAction::OperatorSet(OperatorSetUpdate::new(
            vec![],
            vec![],
        )))
    }

    /// The gate reads the action, not the authority: one Defcon 1 action, opposite answers for two
    /// sessions. Both directions in one test — apart they are halves of a single claim.
    #[test]
    fn defcon_1_is_authorized_for_the_council_and_refused_for_everyone_else() {
        let defcon1 = MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update));

        require_authorized_for_action(Authority::SecurityCouncil, &defcon1).expect("council signs");

        let err = require_authorized_for_action(Authority::StrataAdmin, &defcon1)
            .expect_err("the Strata administrator does not");
        let message = err.to_string();
        assert!(message.contains("Defcon 1"), "{message}");
        assert!(message.contains("Strata Security Council"), "{message}");
    }

    /// AC 2: the same claim for the timelocked lever. Upstream maps both Defcon levels to the
    /// council, so this is a tripwire on upstream rather than on a table of ours — and the error
    /// has to name the role the action requires, since that is what the caller is told.
    #[test]
    fn defcon_3_is_authorized_for_the_council_and_refused_for_everyone_else() {
        let defcon3 = MultisigAction::Update(UpdateAction::Defcon3(Defcon3Update));

        require_authorized_for_action(Authority::SecurityCouncil, &defcon3).expect("council signs");

        let err = require_authorized_for_action(Authority::StrataAdmin, &defcon3)
            .expect_err("the Strata administrator does not");
        let message = err.to_string();
        assert!(message.contains("Defcon 3"), "{message}");
        assert!(message.contains("Strata Security Council"), "{message}");
    }

    /// AC 12: two actions on the Strata Security Council resolve to different depths — the
    /// distinguishing case a per-authority mapping cannot produce.
    #[test]
    fn defcon_1_and_defcon_3_resolve_to_different_depths_on_one_authority() {
        let mut depths = uniform_confirmation_depths(NON_ZERO_BASELINE);
        depths.defcon3 = 7;

        // Tripwire for the composition in docs/specs/security-council-defcon-phase-1.md §4: we hold
        // no local copy of upstream's table, so this is what catches upstream giving Defcon 1 a
        // configurable depth. Every field is non-zero, so `None` here can only come from the
        // hardcoded arm.
        assert!(depths.get(UpdateTxType::Defcon1).is_none());

        let defcon1 = MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update));
        let defcon3 = MultisigAction::Update(UpdateAction::Defcon3(Defcon3Update));

        assert_eq!(depth_for_action(&defcon1, |t| depths.get(t)), 0);
        assert_eq!(depth_for_action(&defcon3, |t| depths.get(t)), 7);
    }

    /// The depth follows the action, not the authority: both of these are created by the Strata
    /// administrator, and the retired per-authority mapping gave them the same answer.
    #[test]
    fn two_actions_of_one_authority_resolve_to_their_own_depths() {
        let mut depths = uniform_confirmation_depths(NON_ZERO_BASELINE);
        depths.strata_admin_multisig_update = 11;
        depths.operator_update = 23;

        assert_eq!(depth_for_action(&signer_update(), |t| depths.get(t)), 11);
        assert_eq!(
            depth_for_action(&operator_set_update(), |t| depths.get(t)),
            23
        );
    }

    /// A cancel is never enqueued — it applies when it confirms — so it carries no lock period,
    /// not the lock period of the update it targets.
    #[test]
    fn cancel_resolves_to_zero() {
        let mut depths = uniform_confirmation_depths(NON_ZERO_BASELINE);
        depths.defcon3 = 7;

        let cancel =
            MultisigAction::Cancel(CancelAction::new(0, UpdateAction::Defcon3(Defcon3Update)));

        assert_eq!(depth_for_action(&cancel, |t| depths.get(t)), 0);
    }

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
            AuthorityAsmSupport::Supported
        );
        assert_eq!(
            authority_asm_support(SecurityCouncil),
            AuthorityAsmSupport::Supported
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

    /// The answer the proposal DTO carries, through the function the handlers actually call. The
    /// depth mapping itself is pinned by the tests above; this pins the wire answer and the two
    /// ways it degrades — neither of which may produce a cancel affordance.
    #[test]
    fn cancelability_follows_the_depth_and_degrades_to_no_affordance() {
        let mut depths = uniform_confirmation_depths(NON_ZERO_BASELINE);
        depths.defcon3 = 7;
        let resolver = ConfirmationDepthResolver::Mock(depths);

        let defcon3 = action_codec::test_fixture_defcon_3_action_hex();
        let defcon1 = action_codec::test_fixture_defcon_1_action_hex();

        assert!(resolver.is_cancelable_for_hex(&defcon3));
        assert!(!resolver.is_cancelable_for_hex(&defcon1));
        assert!(!resolver.is_cancelable_for_hex("not-an-action"));
        assert!(!ConfirmationDepthResolver::Unavailable.is_cancelable_for_hex(&defcon3));
    }
}
