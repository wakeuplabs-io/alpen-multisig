//! Persistence contracts defined by the application layer.
//!
//! Concrete implementations live in `crate::infrastructure`.

use crate::domain::proposal::{ActionId, Proposal, ProposalStatus};
use crate::error::AppError;

/// Persistence contract for proposals.
#[async_trait::async_trait]
pub(crate) trait ProposalRepository: Send + Sync {
    /// Store a new proposal. Fails if ActionId already exists.
    async fn save_proposal(&self, proposal: Proposal) -> Result<(), AppError>;

    /// Find a proposal by ActionId.
    async fn find_by_action_id(&self, action_id: &ActionId) -> Result<Option<Proposal>, AppError>;

    /// Append one signature to an existing proposal.
    async fn add_signature(
        &self,
        action_id: &ActionId,
        signer_pubkey: &str,
        signature_hex: &str,
    ) -> Result<Option<Proposal>, AppError>;

    /// List proposals, optionally filtered by status.
    async fn list_by_status(
        &self,
        status: Option<ProposalStatus>,
    ) -> Result<Vec<Proposal>, AppError>;
}
