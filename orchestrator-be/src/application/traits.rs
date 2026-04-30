//! Persistence contracts defined by the application layer.
//!
//! Concrete implementations live in `crate::infrastructure`.

use crate::domain::proposal::{ActionId, Proposal, ProposalStatus};
use crate::error::AppError;

/// Persistence contract for proposals.
pub(crate) trait ProposalRepository: Send + Sync {
    /// Store a new proposal. Fails if ActionId already exists.
    fn save_proposal(&mut self, proposal: Proposal) -> Result<(), AppError>;

    /// Find a proposal by ActionId (immutable reference).
    fn find_by_action_id(&self, action_id: &ActionId) -> Option<&Proposal>;

    /// Find a proposal by ActionId (mutable reference, for adding signatures).
    fn find_by_action_id_mut(&mut self, action_id: &ActionId) -> Option<&mut Proposal>;

    /// List proposals, optionally filtered by status.
    fn list_by_status(&self, status: Option<ProposalStatus>) -> Vec<&Proposal>;
}
