use crate::{
    application::proposals,
    domain::proposal::{ActionId, Proposal, ProposalSignature, ProposalStatus},
    error::{AppError, Result},
    handlers::auth_session::AuthenticatedSession,
    infrastructure::asm_role_membership,
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

pub async fn list_proposals(
    State(state): State<AppState>,
    _auth: AuthenticatedSession,
    Query(query): Query<ListProposalsQuery>,
) -> Result<Json<ProposalListResponse>> {
    let proposals = proposals::list_proposals(state.repo.as_ref(), query.status).await?;

    Ok(Json(ProposalListResponse { proposals }))
}

pub async fn get_proposal(
    State(state): State<AppState>,
    _auth: AuthenticatedSession,
    Path(action_id): Path<String>,
) -> Result<Json<Proposal>> {
    let proposal = proposals::get_update_action(state.repo.as_ref(), &ActionId(action_id)).await?;

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
