//! Orchestrator client trait and associated error type.
//!
//! The trait lives here (application layer) so application logic can depend on
//! the abstraction without importing the concrete HTTP implementation.

use crate::domain::proposal::{ApproveActionRequest, CreateProposalRequest, Proposal};

// ─── Errors ───────────────────────────────────────────────────────────────────

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

// ─── Trait ────────────────────────────────────────────────────────────────────

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
