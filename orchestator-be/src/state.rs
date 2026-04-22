use crate::config::Config;
use crate::domain::session::Session;
use crate::infrastructure::memory_repo::InMemoryProposalRepository;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub repo: Arc<RwLock<InMemoryProposalRepository>>,
    pub sessions: Arc<RwLock<HashMap<String, Session>>>,
    pub used_nonces: Arc<RwLock<HashSet<[u8; 32]>>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            repo: Arc::new(RwLock::new(InMemoryProposalRepository::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            used_nonces: Arc::new(RwLock::new(HashSet::new())),
        }
    }
}
