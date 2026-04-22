use crate::infrastructure::memory_repo::InMemoryProposalRepository;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<RwLock<InMemoryProposalRepository>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            repo: Arc::new(RwLock::new(InMemoryProposalRepository::new())),
        }
    }
}
