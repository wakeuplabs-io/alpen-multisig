use crate::{
    application::proposals,
    domain::proposal::{ActionId, Proposal, ProposalSignature, ProposalStatus},
    error::{AppError, Result},
    handlers::auth_session::AuthenticatedSession,
    infrastructure::{action_codec, asm_role_membership},
    state::AppState,
};

#[derive(Debug, Serialize)]
pub struct NextSeqNoResponse {
    pub next_seq_no: u64,
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
    let last_seqno =
        asm_role_membership::last_seqno_for_authority(&state.asm_rpc_url, auth.authority).await?;
    Ok(Json(NextSeqNoResponse {
        next_seq_no: last_seqno + 1,
    }))
}

#[tracing::instrument(skip(state, auth), fields(authority = ?auth.authority))]
pub async fn list_proposals(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Query(query): Query<ListProposalsQuery>,
) -> Result<Json<ProposalListResponse>> {
    let proposals =
        proposals::list_proposals(state.repo.as_ref(), auth.authority, query.status).await?;

    Ok(Json(ProposalListResponse { proposals }))
}

#[tracing::instrument(skip(state, auth), fields(action_id, authority = ?auth.authority))]
pub async fn get_proposal(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
) -> Result<Json<Proposal>> {
    let proposal =
        proposals::get_update_action(state.repo.as_ref(), auth.authority, &ActionId(action_id))
            .await?;

    Ok(Json(proposal))
}

pub async fn approve_action(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
    Json(body): Json<ApproveActionRequest>,
) -> Result<Json<Proposal>> {
    if !body.signer_pubkey.eq_ignore_ascii_case(&auth.signer_pubkey) {
        return Err(AppError::Unauthorized);
    }
    let sig = ProposalSignature {
        signer_pubkey: body.signer_pubkey,
        signature_hex: body.signature_hex,
    };

    let proposal = proposals::approve_action(
        state.repo.as_ref(),
        proposals::SessionContext {
            authority: auth.authority,
            signer_pubkey: &auth.signer_pubkey,
        },
        &ActionId(action_id),
        &sig,
    )
    .await?;

    Ok(Json(proposal))
}

/// Coordination-only: desktop claims broadcast before local commit/reveal (P-066).
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
pub async fn report_broadcast_progress(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
    Json(body): Json<ReportBroadcastProgressBody>,
) -> Result<Json<Proposal>> {
    let action_id = ActionId(action_id);
    let proposal = proposals::report_broadcast_progress(
        state.repo.as_ref(),
        auth.authority,
        &action_id,
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
