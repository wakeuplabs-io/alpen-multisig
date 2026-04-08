use axum::{Json, extract::{Path, State}};
use serde::{Deserialize, Serialize};
use crate::{domain::proposal::ProposalSignature, error::Result, state::AppState};

#[derive(Debug, Deserialize)]
pub struct SubmitSignatureRequest {
	pub signer_pubkey: String,
	pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct SubmitSignatureResponse {
	pub signature_id: String,
	pub quorum_reached: bool,
}

pub async fn submit_signature(
	State(_state): State<AppState>,
	Path(_action_id): Path<String>,
	Json(_body): Json<SubmitSignatureRequest>,
) -> Result<Json<SubmitSignatureResponse>> {
	todo!("validate signature, prevent duplicates, check quorum")
}

#[derive(Debug, Serialize)]
pub struct SignatureListResponse {
	pub signatures: Vec<ProposalSignature>,
}

pub async fn list_signatures(
	State(_state): State<AppState>,
	Path(_action_id): Path<String>,
) -> Result<Json<SignatureListResponse>> {
	todo!("list signatures for proposal, scoped to session authority")
}
