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
use crate::infrastructure::asm_role_membership::threshold_for_authority;

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
    if !sig
        .signer_pubkey
        .eq_ignore_ascii_case(session.signer_pubkey)
    {
        return Err(AppError::Unauthorized);
    }
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

/// List proposals for one authority, optionally filtered by status.
pub(crate) async fn list_proposals(
    repo: &dyn ProposalRepository,
    authority: Authority,
    status: Option<ProposalStatus>,
) -> Result<Vec<Proposal>, AppError> {
    repo.list_by_status(authority, status).await
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
        if !crate::infrastructure::asm_enactment::is_proposal_enacted_on_asm(
            asm_rpc_url,
            proposal.authority,
            proposal.seq_no,
            &proposal.action_hex,
        )
        .await?
        {
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

    if !crate::infrastructure::asm_enactment::is_proposal_enacted_on_asm(
        asm_rpc_url,
        proposal.authority,
        proposal.seq_no,
        &proposal.action_hex,
    )
    .await?
    {
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
    Ok(())
}

/// Persist broadcast sub-status reported by the desktop after local Bitcoin submission.
pub(crate) async fn report_broadcast_progress(
    repo: &dyn ProposalRepository,
    asm_rpc_url: &str,
    session_authority: Authority,
    action_id: &ActionId,
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

    repo.update_broadcast_status(
        action_id,
        broadcast_status,
        proposal_status,
        body.commit_txid.as_deref(),
        body.reveal_txid.as_deref(),
        body.broadcast_error.as_deref(),
    )
    .await?
    .ok_or(AppError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::memory_repo::InMemoryProposalRepository;

    fn new_repo() -> InMemoryProposalRepository {
        InMemoryProposalRepository::new()
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
