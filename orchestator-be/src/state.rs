use crate::application::repository::InMemoryProposalRepository;
use crate::config::Config;
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
