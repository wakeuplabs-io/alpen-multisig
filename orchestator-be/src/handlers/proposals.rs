use crate::{
    application,
    domain::proposal::{Proposal, QuorumStatus},
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
    let _proposals = application::list_proposals(todo!("extract authority from session"), None)?;
    todo!("map proposals to ProposalListResponse")
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
    Json(body): Json<CreateProposalRequest>,
) -> Result<Json<CreateProposalResponse>> {
    let (_action_id, _proposal) = application::create_proposal(
        todo!("extract authority from session"),
        body.seq_no,
        body.action_payload,
    )?;
    todo!("map to CreateProposalResponse")
}

pub async fn get_proposal(
    State(_state): State<AppState>,
    Path(action_id): Path<String>,
) -> Result<Json<Proposal>> {
    let proposal = application::get_proposal(&action_id)?;
    Ok(Json(proposal))
}
