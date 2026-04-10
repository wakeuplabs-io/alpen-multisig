//! Proposal repository — persistence abstraction for proposals and signatures.
//!
//! Defines the storage contract and provides an in-memory implementation
//! for POC/testing. Postgres implementation will be added in a future slice.

use crate::application::proposals::{ActionId, Proposal, ProposalStatus};
use crate::error::AppError;
use std::collections::HashMap;

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

/// In-memory implementation for POC and testing.
pub(crate) struct InMemoryProposalRepository {
    proposals: HashMap<ActionId, Proposal>,
}

impl InMemoryProposalRepository {
    pub(crate) fn new() -> Self {
        Self {
            proposals: HashMap::new(),
        }
    }
}

impl ProposalRepository for InMemoryProposalRepository {
    fn save_proposal(&mut self, proposal: Proposal) -> Result<(), AppError> {
        if self.proposals.contains_key(&proposal.action_id) {
            return Err(AppError::Conflict("proposal already exists".to_string()));
        }
        self.proposals.insert(proposal.action_id.clone(), proposal);
        Ok(())
    }

    fn find_by_action_id(&self, action_id: &ActionId) -> Option<&Proposal> {
        self.proposals.get(action_id)
    }

    fn find_by_action_id_mut(&mut self, action_id: &ActionId) -> Option<&mut Proposal> {
        self.proposals.get_mut(action_id)
    }

    fn list_by_status(&self, status: Option<ProposalStatus>) -> Vec<&Proposal> {
        self.proposals
            .values()
            .filter(|p| status.map_or(true, |s| p.status == s))
            .collect()
    }
}
