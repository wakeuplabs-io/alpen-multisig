//! Persistence contracts defined by the application layer.
//!
//! Concrete implementations live in `crate::infrastructure`.

use crate::domain::proposal::{ActionId, BroadcastStatus, Proposal, ProposalStatus};
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

    /// Atomically transition broadcast_status from `Idle` to `CommitBroadcasted`.
    ///
    /// Returns `Ok(proposal)` on success, `Err(Conflict)` if the proposal is not
    /// in the `Idle` state (i.e. another caller already claimed it).
    async fn claim_broadcast(&self, action_id: &ActionId) -> Result<Proposal, AppError>;

    /// Update broadcast sub-status and related txids/error for an approved proposal.
    async fn update_broadcast_status(
        &self,
        action_id: &ActionId,
        status: BroadcastStatus,
        proposal_status: Option<ProposalStatus>,
        commit_txid: Option<&str>,
        reveal_txid: Option<&str>,
        error: Option<&str>,
    ) -> Result<Option<Proposal>, AppError>;
}
