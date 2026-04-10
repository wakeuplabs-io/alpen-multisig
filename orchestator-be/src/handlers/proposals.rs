use crate::{
    application::proposals::{Proposal, QuorumStatus},
    error::Result,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

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

pub async fn list_proposals(State(_state): State<AppState>) -> Result<Json<ProposalListResponse>> {
    todo!("wire to application::proposals::list_proposals with repository from state")
}

#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
    pub seq_no: u64,
    /// Hex-encoded MultisigAction payload.
    pub action_hex: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
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
    todo!("wire to application::proposals::create_update_action with repository from state")
}

pub async fn get_proposal(
    State(_state): State<AppState>,
    Path(_action_id): Path<String>,
) -> Result<Json<Proposal>> {
    todo!("wire to application::proposals::get_update_action with repository from state")
}
