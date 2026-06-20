//! Orchestrator client contract — trait + request/error types.
//!
//! Concrete HTTP implementation lives in `crate::infrastructure::orchestrator_client`.

use crate::domain::proposal::Proposal;
use serde::{Deserialize, Serialize};

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

/// Explicit pending → approved coordination transition (P-012).
#[derive(Debug, Serialize)]
pub struct TransitionProposalRequest {
    pub proposal_status: String,
}

#[derive(Debug, Deserialize)]
pub struct ProposalListResponse {
    pub proposals: Vec<Proposal>,
}

#[derive(Debug, Serialize)]
pub struct StartOrchestratorAuthRequest {
    pub authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorAuthChallenge {
    pub challenge_id: String,
    pub challenge_hex: String,
    pub challenge_message: String,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct CompleteOrchestratorAuthRequest {
    pub challenge_id: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
    pub signature_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorAuthSession {
    pub token: String,
    pub authority: String,
    pub signer_pubkey: String,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct NextSeqNoResponse {
    pub next_seq_no: u64,
}

/// Pre-broadcast guard response for Cancel proposals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTargetStatusResponse {
    pub target_queued: bool,
}

/// Request to create a cancel proposal for an approved target.
#[derive(Debug, Serialize)]
pub struct CreateCancelProposalRequest {
    pub seq_no: u64,
    pub action_hex: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
}

/// Desktop-reported broadcast progress (coordination only).
#[derive(Debug, Clone, Serialize)]
pub struct ReportBroadcastProgressRequest {
    pub broadcast_status: String,
    pub proposal_status: Option<String>,
    pub commit_txid: Option<String>,
    pub reveal_txid: Option<String>,
    pub broadcast_error: Option<String>,
}

/// Abstracts the orchestrator HTTP API.
#[async_trait::async_trait]
pub trait OrchestratorClient: Send + Sync {
    async fn auth_challenge(
        &self,
        request: StartOrchestratorAuthRequest,
    ) -> Result<OrchestratorAuthChallenge, OrchestratorError>;

    async fn auth_verify(
        &self,
        request: CompleteOrchestratorAuthRequest,
    ) -> Result<OrchestratorAuthSession, OrchestratorError>;

    async fn auth_logout(&self) -> Result<(), OrchestratorError>;

    /// Create a new proposal with the first signature.
    async fn create_proposal(
        &self,
        request: CreateProposalRequest,
    ) -> Result<Proposal, OrchestratorError>;

    /// Get full details of a specific proposal.
    async fn get_proposal(&self, action_id: &str) -> Result<Proposal, OrchestratorError>;

    /// Pre-broadcast guard for a Cancel proposal: is its target action still queued on the ASM?
    async fn get_cancel_target_status(
        &self,
        action_id: &str,
    ) -> Result<CancelTargetStatusResponse, OrchestratorError>;

    /// Submit an approval signature for an existing proposal.
    async fn approve_action(
        &self,
        action_id: &str,
        request: ApproveActionRequest,
    ) -> Result<Proposal, OrchestratorError>;

    /// Persist explicit pending → approved after quorum (P-012).
    async fn transition_to_approved(
        &self,
        action_id: &str,
        request: TransitionProposalRequest,
    ) -> Result<Proposal, OrchestratorError>;

    /// List proposals, optionally filtered by status.
    async fn list_proposals(
        &self,
        status: Option<&str>,
    ) -> Result<Vec<Proposal>, OrchestratorError>;

    /// Get the next valid sequence number for the authenticated authority.
    async fn get_next_seq_no(&self) -> Result<u64, OrchestratorError>;

    /// Claim broadcast coordination slot before desktop submits to Bitcoin (P-066).
    async fn claim_broadcast(&self, action_id: &str) -> Result<Proposal, OrchestratorError>;

    /// Report broadcast sub-status after local Bitcoin steps (P-066).
    async fn report_broadcast_progress(
        &self,
        action_id: &str,
        request: ReportBroadcastProgressRequest,
    ) -> Result<Proposal, OrchestratorError>;

    /// Create a cancel proposal for an approved target (idempotent).
    async fn create_cancel_proposal(
        &self,
        target_action_id: &str,
        request: CreateCancelProposalRequest,
    ) -> Result<Proposal, OrchestratorError>;
}
