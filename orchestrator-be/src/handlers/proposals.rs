use crate::{
    application::proposals,
    domain::proposal::{ActionId, Proposal, ProposalSignature, ProposalStatus},
    error::{AppError, Result},
    handlers::auth_session::AuthenticatedSession,
    infrastructure::{action_codec, asm_role_membership},
    state::AppState,
};

// ─── Extended response types ─────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CancelProposalSummary {
    pub action_id: ActionId,
    pub status: ProposalStatus,
    pub signatures: Vec<ProposalSignature>,
    pub required_signatures: u16,
}

#[derive(Debug, Serialize)]
pub struct ProposalDetailResponse {
    #[serde(flatten)]
    pub proposal: Proposal,
    pub cancel_proposal: Option<CancelProposalSummary>,
}

#[derive(Debug, Serialize)]
pub struct NextSeqNoResponse {
    pub next_seq_no: u64,
}

#[derive(Debug, Serialize)]
pub struct CancelTargetStatusResponse {
    pub target_queued: bool,
}
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

// ─── Request types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
    pub seq_no: u64,
    pub action_hex: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
}

#[derive(Debug, Deserialize)]
pub struct ApproveActionRequest {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

#[derive(Debug, Deserialize)]
pub struct ListProposalsQuery {
    pub status: Option<ProposalStatus>,
}

#[derive(Debug, Serialize)]
pub struct ProposalListResponse {
    pub proposals: Vec<Proposal>,
}

// ─── Handlers ───────────────────────────────────────────────────────────────

pub async fn create_proposal(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Json(body): Json<CreateProposalRequest>,
) -> Result<(StatusCode, Json<Proposal>)> {
    let sig = ProposalSignature {
        signer_pubkey: body.signer_pubkey,
        signature_hex: body.signature_hex,
    };

    action_codec::decode_multisig_action_hex(&body.action_hex).map_err(AppError::BadRequest)?;

    let required_signatures =
        asm_role_membership::threshold_for_authority(&state.asm_rpc_url, auth.authority).await?;

    let proposal = proposals::create_update_action(
        state.repo.as_ref(),
        proposals::SessionContext {
            authority: auth.authority,
            signer_pubkey: &auth.signer_pubkey,
        },
        body.seq_no,
        &body.action_hex,
        &sig,
        required_signatures,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(proposal)))
}

pub async fn get_next_seq_no(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
) -> Result<Json<NextSeqNoResponse>> {
    let next_seq_no = proposals::next_seq_no_for_authority(
        state.repo.as_ref(),
        &state.asm_rpc_url,
        auth.authority,
    )
    .await?;
    Ok(Json(NextSeqNoResponse { next_seq_no }))
}

#[tracing::instrument(skip(state, auth), fields(authority = ?auth.authority))]
pub async fn list_proposals(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Query(query): Query<ListProposalsQuery>,
) -> Result<Json<ProposalListResponse>> {
    proposals::reconcile_enacted_for_authority(
        state.repo.as_ref(),
        &state.asm_rpc_url,
        auth.authority,
    )
    .await?;

    let raw = proposals::list_proposals(state.repo.as_ref(), auth.authority, query.status).await?;

    let mut checked = Vec::with_capacity(raw.len());
    for p in raw {
        checked.push(
            proposals::expire_if_overdue(state.repo.as_ref(), p, state.proposal_expiry_days)
                .await?,
        );
    }

    Ok(Json(ProposalListResponse { proposals: checked }))
}

#[tracing::instrument(skip(state, auth), fields(action_id, authority = ?auth.authority))]
pub async fn get_proposal(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
) -> Result<Json<ProposalDetailResponse>> {
    let action_id = ActionId(action_id);
    proposals::reconcile_enacted_for_action(
        state.repo.as_ref(),
        &state.asm_rpc_url,
        auth.authority,
        &action_id,
    )
    .await?;
    proposals::reconcile_update_id_in_queue(state.repo.as_ref(), &state.asm_rpc_url, &action_id)
        .await?;

    let proposal =
        proposals::get_update_action(state.repo.as_ref(), auth.authority, &action_id).await?;
    let proposal =
        proposals::expire_if_overdue(state.repo.as_ref(), proposal, state.proposal_expiry_days)
            .await?;

    let cancel_proposal = state
        .repo
        .find_cancel_for_target(&action_id)
        .await?
        .map(|c| CancelProposalSummary {
            action_id: c.action_id,
            status: c.status,
            signatures: c.signatures,
            required_signatures: c.required_signatures,
        });

    Ok(Json(ProposalDetailResponse {
        proposal,
        cancel_proposal,
    }))
}

/// Pre-broadcast guard for Cancel proposals: reports whether the target proposal is still
/// `Approved` in our records. If it is not (e.g. already `Enacted` via the normal flow), the
/// ASM would reject the Cancel tx and the UI should not allow broadcasting it.
#[tracing::instrument(skip(state, auth), fields(action_id, authority = ?auth.authority))]
pub async fn get_cancel_target_status(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
) -> Result<Json<CancelTargetStatusResponse>> {
    let action_id = ActionId(action_id);
    let target_queued =
        proposals::get_cancel_target_status(state.repo.as_ref(), auth.authority, &action_id)
            .await?;

    Ok(Json(CancelTargetStatusResponse { target_queued }))
}

#[derive(Debug, Deserialize)]
pub struct CreateCancelProposalRequest {
    pub seq_no: u64,
    pub action_hex: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
}

pub async fn create_cancel_proposal(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
    Json(body): Json<CreateCancelProposalRequest>,
) -> Result<Json<Proposal>> {
    if !body.signer_pubkey.eq_ignore_ascii_case(&auth.signer_pubkey) {
        return Err(AppError::Unauthorized);
    }

    let proposal = proposals::create_cancel_proposal(
        state.repo.as_ref(),
        &state.asm_rpc_url,
        ActionId(action_id),
        body.seq_no,
        &body.action_hex,
        &auth.signer_pubkey,
        &body.signature_hex,
    )
    .await?;

    Ok(Json(proposal))
}

#[tracing::instrument(skip(state, auth, body), fields(action_id, authority = ?auth.authority))]
pub async fn approve_action(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
    Json(body): Json<ApproveActionRequest>,
) -> Result<Json<Proposal>> {
    let sig = ProposalSignature {
        signer_pubkey: body.signer_pubkey,
        signature_hex: body.signature_hex,
    };

    let proposal = proposals::approve_action(
        state.repo.as_ref(),
        proposals::SessionContext {
            authority: auth.authority,
            signer_pubkey: &sig.signer_pubkey,
        },
        &ActionId(action_id),
        &sig,
    )
    .await?;

    Ok(Json(proposal))
}

#[derive(Debug, Deserialize)]
pub struct PatchProposalBody {
    pub proposal_status: String,
}

/// Explicit pending → approved transition (P-012 / ADR-006).
#[tracing::instrument(skip(state, auth, body), fields(action_id, authority = ?auth.authority))]
pub async fn patch_proposal(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
    Json(body): Json<PatchProposalBody>,
) -> Result<Json<Proposal>> {
    if body.proposal_status != "approved" {
        return Err(AppError::BadRequest(format!(
            "unsupported proposal_status: {}",
            body.proposal_status
        )));
    }

    let proposal = proposals::transition_to_approved(
        state.repo.as_ref(),
        proposals::SessionContext {
            authority: auth.authority,
            signer_pubkey: &auth.signer_pubkey,
        },
        &state.asm_rpc_url,
        &ActionId(action_id),
    )
    .await?;

    Ok(Json(proposal))
}

/// Coordination-only: desktop claims broadcast before local commit/reveal (P-066).
#[tracing::instrument(skip(state, auth), fields(action_id, authority = ?auth.authority))]
pub async fn claim_broadcast(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
) -> Result<Json<Proposal>> {
    let action_id = ActionId(action_id);
    let proposal = proposals::claim_broadcast_coordination(
        state.repo.as_ref(),
        auth.authority,
        &state.asm_rpc_url,
        &action_id,
    )
    .await?;
    Ok(Json(proposal))
}

#[derive(Debug, Deserialize)]
pub struct ReportBroadcastProgressBody {
    pub broadcast_status: String,
    pub proposal_status: Option<String>,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
    pub broadcast_error: Option<String>,
}

/// Coordination-only: desktop reports txids / sub-status after local Bitcoin steps (P-066).
#[tracing::instrument(skip(state, auth, body), fields(action_id, authority = ?auth.authority))]
pub async fn report_broadcast_progress(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
    Json(body): Json<ReportBroadcastProgressBody>,
) -> Result<Json<Proposal>> {
    let action_id = ActionId(action_id);
    let proposal = proposals::report_broadcast_progress(
        state.repo.as_ref(),
        &state.asm_rpc_url,
        auth.authority,
        &action_id,
        state.btc_client.as_ref(),
        proposals::ReportBroadcastProgressRequest {
            broadcast_status: body.broadcast_status,
            proposal_status: body.proposal_status,
            commit_txid: body.commit_txid,
            reveal_txid: body.reveal_txid,
            broadcast_error: body.broadcast_error,
        },
    )
    .await?;
    Ok(Json(proposal))
}
