use crate::{
    application::proposals,
    domain::{
        authority::Authority,
        proposal::{ActionId, Proposal, ProposalSignature, ProposalStatus},
    },
    error::{AppError, Result},
    middleware::ValidSession,
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

// ─── Request types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
    pub authority: Authority,
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
    _session: ValidSession,
    State(state): State<AppState>,
    Json(body): Json<CreateProposalRequest>,
) -> Result<(StatusCode, Json<Proposal>)> {
    let sig = ProposalSignature {
        signer_pubkey: body.signer_pubkey,
        signature_hex: body.signature_hex,
    };

    let mut repo = state
        .repo
        .write()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;

    let proposal = proposals::create_update_action(
        &mut *repo,
        body.authority,
        body.seq_no,
        &body.action_hex,
        &sig,
    )?;

    Ok((StatusCode::CREATED, Json(proposal)))
}

pub async fn list_proposals(
    _session: ValidSession,
    State(state): State<AppState>,
    Query(query): Query<ListProposalsQuery>,
) -> Result<Json<ProposalListResponse>> {
    let repo = state
        .repo
        .read()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;

    let proposals = proposals::list_proposals(&*repo, query.status);

    Ok(Json(ProposalListResponse { proposals }))
}

pub async fn get_proposal(
    _session: ValidSession,
    State(state): State<AppState>,
    Path(action_id): Path<String>,
) -> Result<Json<Proposal>> {
    let repo = state
        .repo
        .read()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;

    let proposal = proposals::get_update_action(&*repo, &ActionId(action_id))?;

    Ok(Json(proposal))
}

pub async fn approve_action(
    _session: ValidSession,
    State(state): State<AppState>,
    Path(action_id): Path<String>,
    Json(body): Json<ApproveActionRequest>,
) -> Result<Json<Proposal>> {
    let sig = ProposalSignature {
        signer_pubkey: body.signer_pubkey,
        signature_hex: body.signature_hex,
    };

    let mut repo = state
        .repo
        .write()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;

    let proposal = proposals::approve_action(&mut *repo, &ActionId(action_id), &sig)?;

    Ok(Json(proposal))
}
