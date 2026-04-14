//! In-memory proposal repository for POC and testing.

use crate::application::traits::ProposalRepository;
use crate::domain::proposal::{ActionId, Proposal, ProposalStatus};
use crate::error::AppError;
use std::collections::HashMap;

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
            .filter(|p| status.is_none_or(|s| p.status == s))
            .collect()
    }
}
