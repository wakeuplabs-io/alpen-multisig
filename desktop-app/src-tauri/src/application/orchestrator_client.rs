//! Orchestrator client contract — trait + request/error types.
//!
//! Concrete HTTP implementation lives in `crate::infrastructure::orchestrator_client`.

use crate::domain::proposal::Proposal;
use serde::Serialize;

/// Errors from orchestrator communication.
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("HTTP request failed: {0}")]
    Request(String),
    #[error("Orchestrator returned error {status}: {message}")]
    Backend { status: u16, message: String },
    #[error("Failed to deserialize response: {0}")]
    Deserialization(String),
}

/// Request to create a proposal with initial signature.
#[derive(Debug, Serialize)]
pub struct CreateProposalRequest {
    pub authority: String,
    pub seq_no: u64,
    pub action_hex: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
}

/// Request to approve (add signature to) an existing proposal.
#[derive(Debug, Serialize)]
pub struct ApproveActionRequest {
    pub signer_pubkey: String,
    pub signature_hex: String,
}

/// Abstracts the orchestrator HTTP API.
#[async_trait::async_trait]
pub trait OrchestratorClient: Send + Sync {
    /// Create a new proposal with the first signature.
    async fn create_proposal(
        &self,
        request: CreateProposalRequest,
    ) -> Result<Proposal, OrchestratorError>;

    /// Get full details of a specific proposal.
    async fn get_proposal(&self, action_id: &str) -> Result<Proposal, OrchestratorError>;

    /// Submit an approval signature for an existing proposal.
    async fn approve_action(
        &self,
        action_id: &str,
        request: ApproveActionRequest,
    ) -> Result<Proposal, OrchestratorError>;
}
