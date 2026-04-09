use crate::{application, domain::proposal::ProposalSignature, error::Result, state::AppState};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

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
    Path(action_id): Path<String>,
    Json(body): Json<SubmitSignatureRequest>,
) -> Result<Json<SubmitSignatureResponse>> {
    let result = application::submit_signature(&action_id, &body.signer_pubkey, &body.signature)?;
    Ok(Json(SubmitSignatureResponse {
        signature_id: result.signature_id,
        quorum_reached: result.quorum_reached,
    }))
}

#[derive(Debug, Serialize)]
pub struct SignatureListResponse {
    pub signatures: Vec<ProposalSignature>,
}

pub async fn list_signatures(
    State(_state): State<AppState>,
    Path(action_id): Path<String>,
) -> Result<Json<SignatureListResponse>> {
    let signatures = application::list_signatures(&action_id)?;
    Ok(Json(SignatureListResponse { signatures }))
}
