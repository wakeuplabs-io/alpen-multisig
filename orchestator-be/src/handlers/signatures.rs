use crate::{domain::proposal::ProposalSignature, error::Result, state::AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SubmitSignatureRequest {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

#[derive(Debug, Serialize)]
pub struct SubmitSignatureResponse {
    pub quorum_reached: bool,
}

pub async fn submit_signature(
    State(_state): State<AppState>,
    Path(_action_id): Path<String>,
    Json(_body): Json<SubmitSignatureRequest>,
) -> Result<Json<SubmitSignatureResponse>> {
    todo!("wire to application::proposals::approve_action with repository from state")
}

#[derive(Debug, Serialize)]
pub struct SignatureListResponse {
    pub signatures: Vec<ProposalSignature>,
}

pub async fn list_signatures(
    State(_state): State<AppState>,
    Path(_action_id): Path<String>,
) -> Result<Json<SignatureListResponse>> {
    todo!("wire to application::list_signatures with repository from state")
}
