use crate::config::Config;
use crate::infrastructure::memory_repo::InMemoryProposalRepository;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
#[allow(dead_code)] // config used at startup (main.rs) and planned for future use
pub struct AppState {
    pub config: Config,
    pub repo: Arc<RwLock<InMemoryProposalRepository>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            repo: Arc::new(RwLock::new(InMemoryProposalRepository::new())),
        }
    }
}
