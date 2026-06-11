//! Proposal business logic — CRUD operations over the repository trait.
//!
//! Functions receive a `ProposalRepository` as a parameter (dependency injection).
//! No authentication or quorum detection — those are added in future slices.

use crate::application::traits::ProposalRepository;
use crate::domain::authority::Authority;
use crate::domain::proposal::{
    compute_action_id, ActionId, BroadcastStatus, Proposal, ProposalSignature, ProposalStatus,
    SeqNo,
};
use crate::error::AppError;
use crate::infrastructure::asm_role_membership::{
    lock_period_for_authority, threshold_for_authority, update_id_in_queue_for_action,
};
use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;
use chrono::Utc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionContext<'a> {
    pub authority: Authority,
    pub signer_pubkey: &'a str,
}

/// Canonical lowercase hex for signer pubkeys at every ingress (P-063).
pub(crate) fn normalize_signer_pubkey_hex(hex: &str) -> String {
    hex.trim().to_lowercase()
}

/// Create a new proposal with first signature. Rejects duplicate ActionId.
///
/// Mirrors PRD: `create_update_action(action, seq, sig)`.
pub(crate) async fn create_update_action(
    repo: &dyn ProposalRepository,
    session: SessionContext<'_>,
    seq_no: SeqNo,
    action_hex: &str,
    sig: &ProposalSignature,
    required_signatures: u16,
) -> Result<Proposal, AppError> {
    if !sig
        .signer_pubkey
        .eq_ignore_ascii_case(session.signer_pubkey)
    {
        return Err(AppError::Unauthorized);
    }

    let action_id = compute_action_id(seq_no, action_hex)?;

    let mut first_sig = sig.clone();
    first_sig.signer_pubkey = normalize_signer_pubkey_hex(&sig.signer_pubkey);

    let proposal = Proposal {
        action_id,
        seq_no,
        authority: session.authority,
        status: ProposalStatus::Pending,
        required_signatures,
        action_hex: action_hex.to_string(),
        signatures: vec![first_sig],
        broadcast_status: BroadcastStatus::default(),
        commit_txid: None,
        reveal_txid: None,
        broadcast_error: None,
        target_action_id: None,
        activation_height: None,
        update_id_in_queue: None,
        created_at: Utc::now(),
    };

    repo.save_proposal(proposal.clone()).await?;

    Ok(proposal)
}

/// Add a signature to an existing proposal. Rejects duplicate signer.
///
/// Mirrors PRD: `approve_action(id, sig)`.
pub(crate) async fn approve_action(
    repo: &dyn ProposalRepository,
    session: SessionContext<'_>,
    action_id: &ActionId,
    sig: &ProposalSignature,
) -> Result<Proposal, AppError> {
    let existing = repo.find_by_action_id(action_id).await?;
    let proposal = existing.ok_or(AppError::NotFound)?;
    if proposal.authority != session.authority {
        return Err(AppError::Unauthorized);
    }

    let signer_pubkey = normalize_signer_pubkey_hex(&sig.signer_pubkey);

    let already_signed = proposal
        .signatures
        .iter()
        .any(|s| s.signer_pubkey.eq_ignore_ascii_case(&signer_pubkey));

    if already_signed {
        return Err(AppError::Conflict("signer already signed".to_string()));
    }

    let updated = repo
        .add_signature(action_id, &signer_pubkey, &sig.signature_hex)
        .await?;

    let proposal = updated.ok_or(AppError::NotFound)?;

    Ok(proposal)
}

/// Whether off-chain quorum is satisfied (coordination only — not on-chain validity).
pub(crate) fn has_reached_quorum(proposal: &Proposal) -> bool {
    proposal.signatures.len() >= proposal.required_signatures as usize
}

/// Explicit pending → approved transition (P-012 / ADR-006). Desktop calls after quorum.
pub(crate) async fn transition_to_approved(
    repo: &dyn ProposalRepository,
    session: SessionContext<'_>,
    asm_rpc_url: &str,
    action_id: &ActionId,
) -> Result<Proposal, AppError> {
    let raw = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_proposal_authority(&raw, session.authority)?;

    if raw.status == ProposalStatus::Approved {
        return Ok(raw);
    }

    if raw.status != ProposalStatus::Pending {
        return Err(AppError::Conflict(format!(
            "proposal must be pending to transition to approved (current: {:?})",
            raw.status
        )));
    }

    if !has_reached_quorum(&raw) {
        return Err(AppError::Conflict(format!(
            "quorum not reached: {} of {} signatures collected",
            raw.signatures.len(),
            raw.required_signatures
        )));
    }

    ensure_threshold_snapshot_current(&raw, asm_rpc_url).await?;

    repo.update_broadcast_status(
        action_id,
        raw.broadcast_status,
        Some(ProposalStatus::Approved),
        None,
        None,
        None,
    )
    .await?
    .ok_or(AppError::NotFound)
}

fn require_proposal_authority(proposal: &Proposal, authority: Authority) -> Result<(), AppError> {
    if proposal.authority != authority {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

/// Refuse broadcast when the proposal snapshot disagrees with live ASM threshold (P-035).
pub(crate) async fn ensure_threshold_snapshot_current(
    proposal: &Proposal,
    asm_rpc_url: &str,
) -> Result<(), AppError> {
    let live = threshold_for_authority(asm_rpc_url, proposal.authority).await?;
    if proposal.required_signatures != live {
        return Err(AppError::Conflict(format!(
            "proposal required_signatures ({}) does not match current ASM threshold ({live})",
            proposal.required_signatures
        )));
    }
    Ok(())
}

/// Get proposal by ActionId scoped to the session authority.
pub(crate) async fn get_update_action(
    repo: &dyn ProposalRepository,
    authority: Authority,
    action_id: &ActionId,
) -> Result<Proposal, AppError> {
    let proposal = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_proposal_authority(&proposal, authority)?;
    Ok(proposal)
}

/// Pre-broadcast guard for a Cancel proposal: is its target proposal still `Approved`?
///
/// If the target has already left `Approved` (e.g. it was `Enacted` via the normal flow,
/// or `Canceled`/`Expired`), broadcasting the cancel tx would be rejected by the ASM.
pub(crate) async fn get_cancel_target_status(
    repo: &dyn ProposalRepository,
    authority: Authority,
    action_id: &ActionId,
) -> Result<bool, AppError> {
    let proposal = get_update_action(repo, authority, action_id).await?;

    if !proposal.is_cancel() {
        return Err(AppError::BadRequest(
            "proposal is not a cancel proposal".to_string(),
        ));
    }

    let target_action_id = proposal.target_action_id.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("cancel proposal missing target_action_id"))
    })?;

    let target = repo
        .find_by_action_id(target_action_id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(target.status == ProposalStatus::Approved)
}

/// List proposals for one authority, optionally filtered by status.
pub(crate) async fn list_proposals(
    repo: &dyn ProposalRepository,
    authority: Authority,
    status: Option<ProposalStatus>,
) -> Result<Vec<Proposal>, AppError> {
    repo.list_by_status(authority, status).await
}

/// Lazily expire a pending proposal that has exceeded the 7-day TTL.
///
/// No-op for any non-pending status. Persists the `Expired` transition so subsequent
/// reads also return the updated state.
pub(crate) async fn expire_if_overdue(
    repo: &dyn ProposalRepository,
    proposal: Proposal,
    expiry_days: u64,
) -> Result<Proposal, AppError> {
    if proposal.status != ProposalStatus::Pending {
        return Ok(proposal);
    }
    if proposal.created_at + chrono::Duration::days(expiry_days as i64) > Utc::now() {
        return Ok(proposal);
    }
    repo.update_broadcast_status(
        &proposal.action_id,
        proposal.broadcast_status,
        Some(ProposalStatus::Expired),
        None,
        None,
        None,
    )
    .await?
    .ok_or(AppError::NotFound)
}

// ---------------------------------------------------------------------------
// Broadcast coordination (metadata only — on-chain execution is desktop-owned)
// ---------------------------------------------------------------------------

/// Broadcast requires explicit off-chain `approved` (P-012 — no auto-transition on claim).
fn require_approved(proposal: Proposal) -> Result<Proposal, AppError> {
    if proposal.status == ProposalStatus::Approved {
        return Ok(proposal);
    }

    Err(AppError::Conflict(format!(
        "proposal must be in 'approved' state to broadcast (current: {:?})",
        proposal.status
    )))
}

/// Request body for desktop-reported broadcast progress (coordination only — no Bitcoin RPC).
#[derive(Debug, serde::Deserialize)]
pub struct ReportBroadcastProgressRequest {
    pub broadcast_status: String,
    pub proposal_status: Option<String>,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
    pub broadcast_error: Option<String>,
}

/// Atomically claim broadcast coordination rights (`idle` → `commit_broadcasted`).
pub(crate) async fn claim_broadcast_coordination(
    repo: &dyn ProposalRepository,
    session_authority: Authority,
    asm_rpc_url: &str,
    action_id: &ActionId,
) -> Result<Proposal, AppError> {
    let raw = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_proposal_authority(&raw, session_authority)?;

    let proposal = require_approved(raw)?;
    ensure_threshold_snapshot_current(&proposal, asm_rpc_url).await?;

    repo.claim_broadcast(action_id).await
}

/// Promote `approved` + `reveal_confirmed` proposals when ASM shows enactment post-conditions.
pub(crate) async fn reconcile_enacted_for_authority(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    authority: Authority,
) -> Result<(), AppError> {
    let approved = repo
        .list_by_status(authority, Some(ProposalStatus::Approved))
        .await?;

    for proposal in approved {
        if proposal.broadcast_status != BroadcastStatus::RevealConfirmed {
            continue;
        }
        if proposal.reveal_txid.is_none() {
            continue;
        }
        let enacted = match crate::infrastructure::asm_enactment::is_proposal_enacted_on_asm(
            asm_rpc_url,
            proposal.authority,
            proposal.seq_no,
            &proposal.action_hex,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    action_id = %proposal.action_id.0,
                    "reconcile_enacted_for_authority: ASM enactment check failed: {e}"
                );
                continue;
            }
        };
        if !enacted {
            continue;
        }
        if let Some(target_action_id) = &proposal.target_action_id {
            let applied = repo
                .enact_cancel(&proposal.action_id, target_action_id)
                .await?;
            if !applied {
                repo.update_broadcast_status(
                    &proposal.action_id,
                    BroadcastStatus::RevealConfirmed,
                    Some(ProposalStatus::Expired),
                    None,
                    None,
                    None,
                )
                .await?;
            }
            continue;
        }
        repo.update_broadcast_status(
            &proposal.action_id,
            BroadcastStatus::RevealConfirmed,
            Some(ProposalStatus::Enacted),
            None,
            None,
            None,
        )
        .await?;
        cascade_enact_associated_cancel(repo, &proposal.action_id).await?;
    }
    Ok(())
}

/// Re-populate `update_id_in_queue` on a single proposal if it is still null after RevealConfirmed.
///
/// Non-fatal: logs and returns Ok(()) on any lookup failure so callers can proceed.
pub(crate) async fn reconcile_update_id_in_queue(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    action_id: &ActionId,
) -> Result<(), AppError> {
    let Some(proposal) = repo.find_by_action_id(action_id).await? else {
        return Ok(());
    };

    if proposal.is_cancel()
        || proposal.broadcast_status != BroadcastStatus::RevealConfirmed
        || proposal.update_id_in_queue.is_some()
    {
        return Ok(());
    }

    match update_id_in_queue_for_action(asm_rpc_url, &proposal.action_hex).await {
        Ok(Some(id)) => {
            if let Err(e) = repo.update_update_id_in_queue(action_id, id).await {
                tracing::warn!(
                    action_id = %action_id.0,
                    "reconcile_update_id_in_queue: failed to persist id: {e}"
                );
            }
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                action_id = %action_id.0,
                "reconcile_update_id_in_queue: ASM lookup failed: {e}"
            );
        }
    }

    Ok(())
}

/// Reconcile a single proposal before returning it to the client.
pub(crate) async fn reconcile_enacted_for_action(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    authority: Authority,
    action_id: &ActionId,
) -> Result<(), AppError> {
    let Some(proposal) = repo.find_by_action_id(action_id).await? else {
        return Ok(());
    };
    require_proposal_authority(&proposal, authority)?;

    if proposal.status != ProposalStatus::Approved
        || proposal.broadcast_status != BroadcastStatus::RevealConfirmed
        || proposal.reveal_txid.is_none()
    {
        return Ok(());
    }

    let enacted = match crate::infrastructure::asm_enactment::is_proposal_enacted_on_asm(
        asm_rpc_url,
        proposal.authority,
        proposal.seq_no,
        &proposal.action_hex,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                action_id = %action_id.0,
                "reconcile_enacted_for_action: ASM enactment check failed: {e}"
            );
            return Ok(());
        }
    };
    if !enacted {
        return Ok(());
    }

    if let Some(target_action_id) = &proposal.target_action_id {
        let applied = repo.enact_cancel(action_id, target_action_id).await?;
        if !applied {
            repo.update_broadcast_status(
                action_id,
                BroadcastStatus::RevealConfirmed,
                Some(ProposalStatus::Expired),
                None,
                None,
                None,
            )
            .await?;
        }
        return Ok(());
    }

    repo.update_broadcast_status(
        action_id,
        BroadcastStatus::RevealConfirmed,
        Some(ProposalStatus::Enacted),
        None,
        None,
        None,
    )
    .await?;
    cascade_enact_associated_cancel(repo, action_id).await?;
    Ok(())
}

/// When a target proposal becomes `Enacted` via the normal flow, its associated cancel
/// proposal (if any, and not already terminal) is now moot — mark it `Enacted` too so it
/// stops appearing as actionable.
async fn cascade_enact_associated_cancel(
    repo: &dyn ProposalRepository,
    target_action_id: &ActionId,
) -> Result<(), AppError> {
    let Some(cancel) = repo.find_cancel_for_target(target_action_id).await? else {
        return Ok(());
    };
    if cancel.status == ProposalStatus::Enacted {
        return Ok(());
    }
    repo.update_broadcast_status(
        &cancel.action_id,
        cancel.broadcast_status,
        Some(ProposalStatus::Enacted),
        None,
        None,
        None,
    )
    .await?;
    Ok(())
}

/// Create a cancel proposal for an Approved target, or return the existing one (idempotent).
///
/// `action_hex` encodes `MultisigAction::Cancel` — built by the desktop before signing.
pub(crate) async fn create_cancel_proposal(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    target_action_id: ActionId,
    seq_no: SeqNo,
    action_hex: &str,
    signer_pubkey: &str,
    signature_hex: &str,
) -> Result<Proposal, AppError> {
    let target = repo
        .find_by_action_id(&target_action_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if target.status != ProposalStatus::Approved {
        return Err(AppError::BadRequest(format!(
            "target proposal must be approved to cancel (current: {})",
            target.status
        )));
    }

    if !matches!(
        target.authority,
        Authority::AlpenAdmin | Authority::StrataAdmin
    ) {
        return Err(AppError::BadRequest(format!(
            "cancel is only supported for AlpenAdmin and StrataAdmin (got: {:?})",
            target.authority
        )));
    }

    if let Some(existing) = repo.find_cancel_for_target(&target_action_id).await? {
        return Ok(existing);
    }

    let required_signatures = threshold_for_authority(asm_rpc_url, target.authority).await?;
    let action_id = compute_action_id(seq_no, action_hex)?;
    let normalized_pubkey = normalize_signer_pubkey_hex(signer_pubkey);

    let proposal = Proposal {
        action_id,
        seq_no,
        authority: target.authority,
        status: ProposalStatus::Pending,
        required_signatures,
        action_hex: action_hex.to_string(),
        signatures: vec![ProposalSignature {
            signer_pubkey: normalized_pubkey,
            signature_hex: signature_hex.to_string(),
        }],
        broadcast_status: BroadcastStatus::default(),
        commit_txid: None,
        reveal_txid: None,
        broadcast_error: None,
        target_action_id: Some(target_action_id),
        activation_height: None,
        update_id_in_queue: None,
        created_at: Utc::now(),
    };

    repo.save_proposal(proposal.clone()).await?;
    Ok(proposal)
}

/// Compute and persist `activation_height` after a non-cancel proposal's reveal is confirmed.
///
/// Non-fatal: caller should log and continue on error rather than failing the progress report.
async fn compute_and_store_activation_height(
    proposal: &Proposal,
    repo: &dyn ProposalRepository,
    btc_client: &dyn BitcoinRpcClient,
    asm_rpc_url: &str,
) -> Result<(), AppError> {
    let Some(reveal_txid) = &proposal.reveal_txid else {
        return Ok(());
    };
    let reveal_confirm_block = btc_client.get_block_height_for_txid(reveal_txid).await?;
    let lock_period = lock_period_for_authority(asm_rpc_url, proposal.authority).await?;
    let activation_height = reveal_confirm_block + lock_period;
    repo.update_activation_height(&proposal.action_id, activation_height)
        .await
}

/// Persist broadcast sub-status reported by the desktop after local Bitcoin submission.
pub(crate) async fn report_broadcast_progress(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    session_authority: Authority,
    action_id: &ActionId,
    btc_client: &dyn BitcoinRpcClient,
    body: ReportBroadcastProgressRequest,
) -> Result<Proposal, AppError> {
    let raw = repo
        .find_by_action_id(action_id)
        .await?
        .ok_or(AppError::NotFound)?;
    require_proposal_authority(&raw, session_authority)?;

    let broadcast_status: BroadcastStatus = body.broadcast_status.parse()?;
    let proposal_status = match body.proposal_status.as_deref() {
        None => None,
        Some("pending") => Some(ProposalStatus::Pending),
        Some("approved") => {
            return Err(AppError::BadRequest(
                "use PATCH /proposals/:action_id with proposal_status approved to transition; not via broadcast PATCH"
                    .to_string(),
            ));
        }
        Some("enacted") => {
            if raw.status == ProposalStatus::Enacted {
                None
            } else {
                if raw.broadcast_status != BroadcastStatus::RevealConfirmed {
                    return Err(AppError::Conflict(format!(
                        "proposal must have broadcast_status reveal_confirmed before enacted (current: {})",
                        raw.broadcast_status
                    )));
                }
                if !crate::infrastructure::asm_enactment::is_proposal_enacted_on_asm(
                    asm_rpc_url,
                    raw.authority,
                    raw.seq_no,
                    &raw.action_hex,
                )
                .await?
                {
                    return Err(AppError::Conflict(
                        "ASM state does not yet show proposal enactment".to_string(),
                    ));
                }
                Some(ProposalStatus::Enacted)
            }
        }
        Some("canceled") => Some(ProposalStatus::Canceled),
        Some("expired") => Some(ProposalStatus::Expired),
        Some(other) => {
            return Err(AppError::BadRequest(format!(
                "unknown proposal_status: {other}"
            )));
        }
    };

    let updated = repo
        .update_broadcast_status(
            action_id,
            broadcast_status,
            proposal_status,
            body.commit_txid.as_deref(),
            body.reveal_txid.as_deref(),
            body.broadcast_error.as_deref(),
        )
        .await?
        .ok_or(AppError::NotFound)?;

    if updated.broadcast_status == BroadcastStatus::RevealConfirmed && !updated.is_cancel() {
        if let Err(e) =
            compute_and_store_activation_height(&updated, repo, btc_client, asm_rpc_url).await
        {
            tracing::warn!(
                action_id = %updated.action_id.0,
                "failed to compute activation_height: {e}"
            );
        }
        match update_id_in_queue_for_action(asm_rpc_url, &updated.action_hex).await {
            Ok(Some(id)) => {
                if let Err(e) = repo.update_update_id_in_queue(&updated.action_id, id).await {
                    tracing::warn!(
                        action_id = %updated.action_id.0,
                        "failed to store update_id_in_queue: {e}"
                    );
                }
            }
            Ok(None) => {
                tracing::warn!(
                    action_id = %updated.action_id.0,
                    "update not found in ASM queue after RevealConfirmed"
                );
            }
            Err(e) => {
                tracing::warn!(
                    action_id = %updated.action_id.0,
                    "failed to query ASM for update_id_in_queue: {e}"
                );
            }
        }
    }

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;
    use crate::infrastructure::memory_repo::InMemoryProposalRepository;

    fn new_repo() -> InMemoryProposalRepository {
        InMemoryProposalRepository::new()
    }

    struct MockBitcoinRpcClient {
        block_height: u64,
    }

    #[async_trait::async_trait]
    impl BitcoinRpcClient for MockBitcoinRpcClient {
        async fn estimate_fee_rate_sats_per_vb(
            &self,
            _target_blocks: u16,
        ) -> Result<u64, AppError> {
            Ok(1)
        }

        async fn get_block_height_for_txid(&self, _txid: &str) -> Result<u64, AppError> {
            Ok(self.block_height)
        }
    }

    fn mock_btc() -> MockBitcoinRpcClient {
        MockBitcoinRpcClient {
            block_height: 800_000,
        }
    }

    const ACTION_HEX: &str = "deadbeef";

    fn sig_a() -> ProposalSignature {
        ProposalSignature {
            signer_pubkey: "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
                .to_string(),
            signature_hex: "sig_a".to_string(),
        }
    }

    fn sig_b() -> ProposalSignature {
        ProposalSignature {
            signer_pubkey: "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5"
                .to_string(),
            signature_hex: "sig_b".to_string(),
        }
    }

    #[tokio::test]
    async fn test_create_update_action() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let proposal = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        assert_eq!(proposal.seq_no, 1);
        assert_eq!(
            proposal.authority,
            crate::domain::authority::Authority::StrataAdmin
        );
        assert_eq!(proposal.action_hex, ACTION_HEX);
        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert_eq!(proposal.required_signatures, 2);
        assert_eq!(proposal.signatures.len(), 1);
        assert_eq!(proposal.signatures[0].signer_pubkey, sig.signer_pubkey);

        let expected_id = compute_action_id(1, ACTION_HEX).unwrap();
        assert_eq!(proposal.action_id, expected_id);
    }

    #[tokio::test]
    async fn test_create_duplicate_action_rejected() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let result = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2).await;

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_approve_action() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        let updated = approve_action(&repo, session_b, &created.action_id, &sig_b())
            .await
            .unwrap();

        assert_eq!(updated.signatures.len(), 2);
        assert_eq!(
            updated.signatures[1].signer_pubkey,
            "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5"
        );
    }

    #[tokio::test]
    async fn test_approve_at_quorum_stays_pending_until_explicit_transition() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        let updated = approve_action(&repo, session_b, &created.action_id, &sig_b())
            .await
            .unwrap();

        assert_eq!(updated.signatures.len(), 2);
        assert_eq!(updated.status, ProposalStatus::Pending);
    }

    #[tokio::test]
    async fn test_transition_to_approved_at_quorum() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b.clone(), &created.action_id, &sig_b())
            .await
            .unwrap();

        let approved = transition_to_approved(
            &repo,
            session_b,
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap();

        assert_eq!(approved.status, ProposalStatus::Approved);
    }

    #[tokio::test]
    async fn test_transition_to_approved_fails_below_quorum() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 3)
            .await
            .unwrap();

        let err =
            transition_to_approved(&repo, session, "mock://asm-membership", &created.action_id)
                .await
                .unwrap_err();

        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_transition_to_approved_idempotent() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b.clone(), &created.action_id, &sig_b())
            .await
            .unwrap();

        transition_to_approved(
            &repo,
            session_b.clone(),
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap();
        let again = transition_to_approved(
            &repo,
            session_b,
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap();

        assert_eq!(again.status, ProposalStatus::Approved);
    }

    #[tokio::test]
    async fn test_transition_rejects_threshold_drift() {
        let repo = new_repo();
        let proposal = Proposal {
            action_id: ActionId("drift".to_string()),
            seq_no: 1,
            authority: Authority::StrataAdmin,
            status: ProposalStatus::Pending,
            required_signatures: 99,
            action_hex: ACTION_HEX.to_string(),
            signatures: vec![sig_a(), sig_b()],
            broadcast_status: BroadcastStatus::Idle,
            commit_txid: None,
            reveal_txid: None,
            broadcast_error: None,
            target_action_id: None,
            activation_height: None,
            update_id_in_queue: None,
            created_at: chrono::Utc::now(),
        };
        repo.save_proposal(proposal.clone()).await.unwrap();

        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_a().signer_pubkey,
        };
        let err =
            transition_to_approved(&repo, session, "mock://asm-membership", &proposal.action_id)
                .await
                .unwrap_err();

        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_claim_broadcast_rejects_pending_at_quorum() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b, &created.action_id, &sig_b())
            .await
            .unwrap();

        let err = claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_claim_broadcast_409_when_not_idle() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b, &created.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(
            &repo,
            SessionContext {
                authority: Authority::StrataAdmin,
                signer_pubkey: &sig_b().signer_pubkey,
            },
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap();

        claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap();

        let err = claim_broadcast_coordination(
            &repo,
            Authority::StrataAdmin,
            "mock://asm-membership",
            &created.action_id,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_approve_action_stays_pending_below_threshold() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        // required_signatures = 3, only 2 collected
        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 3)
            .await
            .unwrap();

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        let updated = approve_action(&repo, session_b, &created.action_id, &sig_b())
            .await
            .unwrap();

        assert_eq!(updated.status, ProposalStatus::Pending);
        assert_eq!(updated.signatures.len(), 2);
    }

    #[tokio::test]
    async fn test_approve_duplicate_signer_rejected() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let dup_sig = ProposalSignature {
            signer_pubkey: "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
                .to_string(),
            signature_hex: "different_sig".to_string(),
        };
        let result = approve_action(&repo, session, &created.action_id, &dup_sig).await;

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_approve_nonexistent_proposal() {
        let repo = new_repo();
        let fake_id = ActionId("nonexistent".to_string());

        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_a().signer_pubkey,
        };
        let result = approve_action(&repo, session, &fake_id, &sig_a()).await;

        assert!(matches!(result.unwrap_err(), AppError::NotFound));
    }

    #[tokio::test]
    async fn test_get_update_action() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let fetched = get_update_action(&repo, Authority::StrataAdmin, &created.action_id)
            .await
            .unwrap();

        assert_eq!(fetched.action_id, created.action_id);
        assert_eq!(fetched.action_hex, created.action_hex);
        assert_eq!(fetched.seq_no, created.seq_no);
    }

    #[tokio::test]
    async fn test_get_nonexistent_proposal() {
        let repo = new_repo();
        let fake_id = ActionId("nonexistent".to_string());

        let result = get_update_action(&repo, Authority::StrataAdmin, &fake_id).await;

        assert!(matches!(result.unwrap_err(), AppError::NotFound));
    }

    #[tokio::test]
    async fn test_get_update_action_rejects_wrong_authority() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let result = get_update_action(&repo, Authority::AlpenAdmin, &created.action_id).await;

        assert!(matches!(result.unwrap_err(), AppError::Unauthorized));
    }

    #[tokio::test]
    async fn test_list_proposals() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        create_update_action(&repo, session.clone(), 1, "aa", &sig, 2)
            .await
            .unwrap();
        create_update_action(&repo, session, 2, "bb", &sig, 2)
            .await
            .unwrap();

        let all = list_proposals(&repo, Authority::StrataAdmin, None)
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_list_proposals_scoped_to_authority() {
        let repo = new_repo();
        let sig = sig_a();
        let strata_session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };
        create_update_action(&repo, strata_session, 1, "aa", &sig, 2)
            .await
            .unwrap();

        let alpen_proposal = Proposal {
            action_id: ActionId("alpen_action".to_string()),
            seq_no: 9,
            authority: Authority::AlpenAdmin,
            status: ProposalStatus::Pending,
            required_signatures: 2,
            action_hex: "cc".to_string(),
            signatures: vec![sig.clone()],
            broadcast_status: BroadcastStatus::default(),
            commit_txid: None,
            reveal_txid: None,
            broadcast_error: None,
            target_action_id: None,
            activation_height: None,
            update_id_in_queue: None,
            created_at: chrono::Utc::now(),
        };
        repo.save_proposal(alpen_proposal).await.unwrap();

        let strata_only = list_proposals(&repo, Authority::StrataAdmin, None)
            .await
            .unwrap();
        assert_eq!(strata_only.len(), 1);
        assert_eq!(strata_only[0].authority, Authority::StrataAdmin);
    }

    #[tokio::test]
    async fn test_list_proposals_with_status_filter() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        create_update_action(&repo, session.clone(), 1, "aa", &sig, 2)
            .await
            .unwrap();
        create_update_action(&repo, session, 2, "bb", &sig, 2)
            .await
            .unwrap();

        let pending = list_proposals(&repo, Authority::StrataAdmin, Some(ProposalStatus::Pending))
            .await
            .unwrap();
        assert_eq!(pending.len(), 2);

        let approved = list_proposals(
            &repo,
            Authority::StrataAdmin,
            Some(ProposalStatus::Approved),
        )
        .await
        .unwrap();
        assert_eq!(approved.len(), 0);
    }

    #[tokio::test]
    async fn test_broadcast_rejects_threshold_drift() {
        let proposal = Proposal {
            action_id: ActionId("a".to_string()),
            seq_no: 1,
            authority: Authority::StrataAdmin,
            status: ProposalStatus::Approved,
            required_signatures: 99,
            action_hex: ACTION_HEX.to_string(),
            signatures: vec![sig_a()],
            broadcast_status: BroadcastStatus::Idle,
            commit_txid: None,
            reveal_txid: None,
            broadcast_error: None,
            target_action_id: None,
            activation_height: None,
            update_id_in_queue: None,
            created_at: chrono::Utc::now(),
        };

        let err = ensure_threshold_snapshot_current(&proposal, "mock://asm-membership")
            .await
            .unwrap_err();

        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_approve_rejects_duplicate_signer_case_insensitive() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let upper_pubkey = sig.signer_pubkey.to_uppercase();
        let second = ProposalSignature {
            signer_pubkey: upper_pubkey,
            signature_hex: "other_sig".to_string(),
        };
        let result = approve_action(&repo, session, &created.action_id, &second).await;

        assert!(matches!(result.unwrap_err(), AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_create_rejects_signer_mismatch_against_session() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: "03aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        };

        let err = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    async fn save_reveal_confirmed_approved(repo: &InMemoryProposalRepository) -> ActionId {
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };
        let created = create_update_action(repo, session.clone(), 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(repo, session_b.clone(), &created.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(repo, session_b, "mock://asm-membership", &created.action_id)
            .await
            .unwrap();
        repo.update_broadcast_status(
            &created.action_id,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit"),
            Some("reveal"),
            None,
        )
        .await
        .unwrap();
        created.action_id
    }

    #[tokio::test]
    async fn report_reveal_confirmed_keeps_proposal_approved() {
        let repo = new_repo();
        let action_id = save_reveal_confirmed_approved(&repo).await;

        let updated = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            Authority::StrataAdmin,
            &action_id,
            &mock_btc(),
            ReportBroadcastProgressRequest {
                broadcast_status: "reveal_confirmed".to_string(),
                proposal_status: None,
                commit_txid: Some("commit".to_string()),
                reveal_txid: Some("reveal".to_string()),
                broadcast_error: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.status, ProposalStatus::Approved);
        assert_eq!(updated.broadcast_status, BroadcastStatus::RevealConfirmed);
    }

    #[tokio::test]
    async fn report_enacted_rejected_when_asm_not_enacted() {
        let repo = new_repo();
        let action_id = save_reveal_confirmed_approved(&repo).await;

        let err = report_broadcast_progress(
            &repo,
            "mock://asm-membership",
            Authority::StrataAdmin,
            &action_id,
            &mock_btc(),
            ReportBroadcastProgressRequest {
                broadcast_status: "reveal_confirmed".to_string(),
                proposal_status: Some("enacted".to_string()),
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
            },
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn reconcile_promotes_reveal_confirmed_when_asm_enacted() {
        let repo = new_repo();
        let action_id = save_reveal_confirmed_approved(&repo).await;

        reconcile_enacted_for_authority(&repo, "mock://asm-enacted", Authority::StrataAdmin)
            .await
            .unwrap();

        let proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        assert_eq!(proposal.status, ProposalStatus::Enacted);
    }

    #[tokio::test]
    async fn reconcile_marks_target_canceled_when_cancel_enacted() {
        let repo = new_repo();
        let target = save_approved_proposal(&repo, Authority::StrataAdmin, 1, ACTION_HEX).await;

        let cancel = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            target.action_id.clone(),
            2,
            "cafebabe",
            &sig_a().signer_pubkey,
            "cancel_sig",
        )
        .await
        .unwrap();

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b.clone(), &cancel.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(&repo, session_b, "mock://asm-membership", &cancel.action_id)
            .await
            .unwrap();
        repo.update_broadcast_status(
            &cancel.action_id,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit"),
            Some("reveal"),
            None,
        )
        .await
        .unwrap();

        reconcile_enacted_for_authority(&repo, "mock://asm-enacted", Authority::StrataAdmin)
            .await
            .unwrap();

        let cancel = repo
            .find_by_action_id(&cancel.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cancel.status, ProposalStatus::Enacted);

        let target = repo
            .find_by_action_id(&target.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(target.status, ProposalStatus::Canceled);
    }

    #[tokio::test]
    async fn reconcile_marks_cancel_expired_when_target_already_enacted() {
        let repo = new_repo();
        let target = save_approved_proposal(&repo, Authority::StrataAdmin, 1, ACTION_HEX).await;

        let cancel = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            target.action_id.clone(),
            2,
            "cafebabe",
            &sig_a().signer_pubkey,
            "cancel_sig",
        )
        .await
        .unwrap();

        let session_b = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(&repo, session_b.clone(), &cancel.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(
            &repo,
            session_b.clone(),
            "mock://asm-membership",
            &cancel.action_id,
        )
        .await
        .unwrap();

        // Target reaches RevealConfirmed and gets enacted normally before the cancel does.
        repo.update_broadcast_status(
            &target.action_id,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit"),
            Some("reveal"),
            None,
        )
        .await
        .unwrap();
        reconcile_enacted_for_authority(&repo, "mock://asm-enacted", Authority::StrataAdmin)
            .await
            .unwrap();

        let target = repo
            .find_by_action_id(&target.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(target.status, ProposalStatus::Enacted);

        // The cancel reaches RevealConfirmed afterwards, but its target is no longer Approved —
        // the ASM would have rejected the cancel tx, so it should be marked Expired without
        // touching the already-enacted target.
        repo.update_broadcast_status(
            &cancel.action_id,
            BroadcastStatus::RevealConfirmed,
            None,
            Some("commit"),
            Some("reveal"),
            None,
        )
        .await
        .unwrap();
        reconcile_enacted_for_authority(&repo, "mock://asm-enacted", Authority::StrataAdmin)
            .await
            .unwrap();

        let cancel = repo
            .find_by_action_id(&cancel.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cancel.status, ProposalStatus::Expired);

        let target = repo
            .find_by_action_id(&target.action_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(target.status, ProposalStatus::Enacted);
    }

    // ---------------------------------------------------------------------------
    // create_cancel_proposal tests
    // ---------------------------------------------------------------------------

    async fn save_approved_proposal(
        repo: &InMemoryProposalRepository,
        authority: Authority,
        seq_no: SeqNo,
        action_hex: &str,
    ) -> Proposal {
        let sig = sig_a();
        let session = SessionContext {
            authority,
            signer_pubkey: &sig.signer_pubkey,
        };
        let created = create_update_action(repo, session.clone(), seq_no, action_hex, &sig, 2)
            .await
            .unwrap();
        let session_b = SessionContext {
            authority,
            signer_pubkey: &sig_b().signer_pubkey,
        };
        approve_action(repo, session_b.clone(), &created.action_id, &sig_b())
            .await
            .unwrap();
        transition_to_approved(repo, session_b, "mock://asm-membership", &created.action_id)
            .await
            .unwrap();
        repo.find_by_action_id(&created.action_id)
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn test_create_cancel_proposal_happy_path() {
        let repo = new_repo();
        // StrataAdmin has a mock threshold of 2; use it for cancel creation
        let target = save_approved_proposal(&repo, Authority::StrataAdmin, 1, ACTION_HEX).await;

        let cancel = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            target.action_id.clone(),
            2,
            "cafebabe",
            &sig_a().signer_pubkey,
            "cancel_sig",
        )
        .await
        .unwrap();

        assert_eq!(cancel.status, ProposalStatus::Pending);
        assert_eq!(cancel.authority, Authority::StrataAdmin);
        assert_eq!(cancel.target_action_id, Some(target.action_id));
        assert_eq!(cancel.signatures.len(), 1);
        assert_eq!(cancel.action_hex, "cafebabe");
    }

    #[tokio::test]
    async fn test_create_cancel_proposal_idempotent() {
        let repo = new_repo();
        let target = save_approved_proposal(&repo, Authority::StrataAdmin, 1, ACTION_HEX).await;

        let first = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            target.action_id.clone(),
            2,
            "cafebabe",
            &sig_a().signer_pubkey,
            "cancel_sig",
        )
        .await
        .unwrap();

        let second = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            target.action_id.clone(),
            2,
            "cafebabe",
            &sig_b().signer_pubkey,
            "cancel_sig_2",
        )
        .await
        .unwrap();

        assert_eq!(first.action_id, second.action_id);
        assert_eq!(
            second.signatures.len(),
            1,
            "idempotent: no new sig appended"
        );
    }

    #[tokio::test]
    async fn test_create_cancel_proposal_rejects_non_approved_target() {
        let repo = new_repo();
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };
        let pending = create_update_action(&repo, session, 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();

        let err = create_cancel_proposal(
            &repo,
            "mock://asm-membership",
            pending.action_id,
            2,
            "cafebabe",
            &sig_a().signer_pubkey,
            "cancel_sig",
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_create_cancel_proposal_rejects_unsupported_authority() {
        // Insert approved proposals directly to avoid threshold_for_authority calls during setup.
        let cases = [
            (
                Authority::SequencerManager,
                ActionId("seq_target".to_string()),
            ),
            (
                Authority::SecurityCouncil,
                ActionId("sec_target".to_string()),
            ),
        ];

        for (authority, target_action_id) in cases {
            let repo = new_repo();
            let target = Proposal {
                action_id: target_action_id.clone(),
                seq_no: 1,
                authority,
                status: ProposalStatus::Approved,
                required_signatures: 2,
                action_hex: ACTION_HEX.to_string(),
                signatures: vec![sig_a(), sig_b()],
                broadcast_status: BroadcastStatus::Idle,
                commit_txid: None,
                reveal_txid: None,
                broadcast_error: None,
                target_action_id: None,
                activation_height: None,
                update_id_in_queue: None,
                created_at: chrono::Utc::now(),
            };
            repo.save_proposal(target).await.unwrap();

            let err = create_cancel_proposal(
                &repo,
                "mock://asm-membership",
                target_action_id,
                99,
                "cafebabe",
                &sig_a().signer_pubkey,
                "cancel_sig",
            )
            .await
            .unwrap_err();

            assert!(
                matches!(err, AppError::BadRequest(_)),
                "expected BadRequest for authority {authority:?}"
            );
        }
    }

    /// P-032 (race): concurrent duplicate approve results in exactly one signature stored.
    #[tokio::test]
    async fn test_concurrent_duplicate_approve_single_signature() {
        use std::sync::Arc;

        let repo = Arc::new(InMemoryProposalRepository::new());
        let sig = sig_a();
        let session = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig.signer_pubkey,
        };

        let created = create_update_action(&*repo, session, 1, ACTION_HEX, &sig, 2)
            .await
            .unwrap();
        let action_id = created.action_id.clone();

        // Two signers: one is sig_b (first signer was sig_a who created the proposal).
        // We attempt two concurrent approvals from sig_b — only one should land.
        let sig_b = sig_b();
        let dup_sig = ProposalSignature {
            signer_pubkey: sig_b.signer_pubkey.clone(),
            signature_hex: "sig_b_dup".to_string(),
        };

        let repo1 = Arc::clone(&repo);
        let repo2 = Arc::clone(&repo);
        let action_id1 = action_id.clone();
        let action_id2 = action_id.clone();
        let session1 = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &sig_b.signer_pubkey,
        };
        let session2 = SessionContext {
            authority: Authority::StrataAdmin,
            signer_pubkey: &dup_sig.signer_pubkey,
        };

        let (r1, r2) = tokio::join!(
            approve_action(&*repo1, session1, &action_id1, &sig_b),
            approve_action(&*repo2, session2, &action_id2, &dup_sig),
        );

        let successes = [r1.is_ok(), r2.is_ok()].iter().filter(|&&ok| ok).count();
        let conflicts = [r1.is_err(), r2.is_err()]
            .iter()
            .filter(|&&err| err)
            .count();
        assert_eq!(successes, 1, "exactly one approve should succeed");
        assert_eq!(
            conflicts, 1,
            "exactly one approve should fail with conflict"
        );

        let final_proposal = repo.find_by_action_id(&action_id).await.unwrap().unwrap();
        let b_sigs: Vec<_> = final_proposal
            .signatures
            .iter()
            .filter(|s| s.signer_pubkey.eq_ignore_ascii_case(&sig_b.signer_pubkey))
            .collect();
        assert_eq!(
            b_sigs.len(),
            1,
            "only one signature from sig_b must be stored"
        );
    }
}
