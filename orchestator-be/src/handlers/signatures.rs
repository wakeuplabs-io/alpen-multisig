use crate::{
    application::proposals::{self, ActionId, Proposal, ProposalSignature},
    error::{AppError, Result},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

// ─── Request / Response types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SubmitSignatureRequest {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

#[derive(Debug, Serialize)]
pub struct SubmitSignatureResponse {
    pub proposal: Proposal,
}

#[derive(Debug, Serialize)]
pub struct SignatureListResponse {
    pub signatures: Vec<ProposalSignature>,
}

// ─── Handlers ───────────────────────────────────────────────────────────────

pub async fn submit_signature(
    State(state): State<AppState>,
    Path(action_id): Path<String>,
    Json(body): Json<SubmitSignatureRequest>,
) -> Result<Json<SubmitSignatureResponse>> {
    let sig = ProposalSignature {
        signer_pubkey: body.signer_pubkey,
        signature_hex: body.signature_hex,
    };

    let mut repo = state
        .repo
        .write()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;

    let proposal = proposals::approve_action(&mut *repo, &ActionId(action_id), &sig)?;

    Ok(Json(SubmitSignatureResponse { proposal }))
}

pub async fn list_signatures(
    State(state): State<AppState>,
    Path(action_id): Path<String>,
) -> Result<Json<SignatureListResponse>> {
    let repo = state
        .repo
        .read()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("repo lock poisoned")))?;

    let proposal = proposals::get_update_action(&*repo, &ActionId(action_id))?;

    Ok(Json(SignatureListResponse {
        signatures: proposal.signatures,
    }))
}
