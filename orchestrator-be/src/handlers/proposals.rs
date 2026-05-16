use crate::{
    application::proposals,
    domain::proposal::{ActionId, Proposal, ProposalSignature, ProposalStatus},
    error::{AppError, Result},
    handlers::auth_session::AuthenticatedSession,
    infrastructure::asm_role_membership,
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

// ─── Broadcast response types ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PrepareBroadcastResponse {
    pub action_id: String,
    pub commit_address: String,
    pub commit_amount_sats: u64,
    pub estimated_fee_sats: u64,
}

#[derive(Debug, Serialize)]
pub struct BroadcastResponse {
    pub action_id: String,
    pub proposal_status: String,
    pub broadcast_status: String,
    pub commit_txid: String,
    pub reveal_txid: String,
}

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

pub async fn list_proposals(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Query(query): Query<ListProposalsQuery>,
) -> Result<Json<ProposalListResponse>> {
    let proposals =
        proposals::list_proposals(state.repo.as_ref(), auth.authority, query.status).await?;

    Ok(Json(ProposalListResponse { proposals }))
}

pub async fn get_proposal(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
) -> Result<Json<Proposal>> {
    let proposal = proposals::get_update_action(
        state.repo.as_ref(),
        auth.authority,
        &ActionId(action_id),
    )
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

pub async fn prepare_broadcast(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
) -> Result<Json<PrepareBroadcastResponse>> {
    let action_id = ActionId(action_id);
    let bundle = proposals::prepare_broadcast_bundle(
        state.repo.as_ref(),
        auth.authority,
        state.btc_client.as_ref(),
        &state.asm_rpc_url,
        &state.operator_keypair,
        state.bitcoin_network,
        &action_id,
    )
    .await?;

    Ok(Json(PrepareBroadcastResponse {
        action_id: action_id.0,
        commit_address: bundle.commit_address,
        commit_amount_sats: bundle.commit_amount_sats,
        estimated_fee_sats: bundle.estimated_fee_sats,
    }))
}

pub async fn execute_broadcast(
    State(state): State<AppState>,
    auth: AuthenticatedSession,
    Path(action_id): Path<String>,
) -> Result<Json<BroadcastResponse>> {
    let action_id = ActionId(action_id);
    let (commit_txid, reveal_txid) = proposals::broadcast_commit_then_reveal(
        state.repo.as_ref(),
        auth.authority,
        state.btc_client.as_ref(),
        &state.asm_rpc_url,
        &state.operator_keypair,
        &action_id,
        state.bitcoin_magic_bytes,
        state.bitcoin_network,
        state.confirm_poll_interval_ms,
        state.confirm_timeout_ms,
    )
    .await?;

    let proposal = state
        .repo
        .find_by_action_id(&action_id)
        .await?
        .ok_or_else(|| AppError::NotFound)?;

    Ok(Json(BroadcastResponse {
        action_id: action_id.0,
        proposal_status: proposal.status.to_string(),
        broadcast_status: proposal.broadcast_status.to_string(),
        commit_txid,
        reveal_txid,
    }))
}
