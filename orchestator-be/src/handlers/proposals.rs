use axum::{Json, extract::{Path, State}};
use serde::{Deserialize, Serialize};
use crate::{domain::proposal::{Proposal, QuorumStatus}, error::Result, state::AppState};

#[derive(Debug, Serialize)]
pub struct ProposalListResponse {
	pub proposals: Vec<ProposalSummary>,
}

#[derive(Debug, Serialize)]
pub struct ProposalSummary {
	pub action_id: String,
	pub seq_no: u64,
	pub status: String,
	pub quorum: QuorumStatus,
	pub expires_at: String,
}

pub async fn list_proposals(
	State(_state): State<AppState>,
) -> Result<Json<ProposalListResponse>> {
	todo!("list proposals scoped to session authority")
}

#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
	pub seq_no: u64,
	/// Serialized MultisigAction payload.
	pub action_payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct CreateProposalResponse {
	pub action_id: String,
	pub proposal: Proposal,
}

pub async fn create_proposal(
	State(_state): State<AppState>,
	Json(_body): Json<CreateProposalRequest>,
) -> Result<Json<CreateProposalResponse>> {
	todo!("derive ActionId, prevent duplicates, store proposal")
}

pub async fn get_proposal(
	State(_state): State<AppState>,
	Path(_action_id): Path<String>,
) -> Result<Json<Proposal>> {
	todo!("fetch proposal by ActionId, scoped to session authority")
}
