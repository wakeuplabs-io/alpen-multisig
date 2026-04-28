use crate::domain::auth::{AuthSession, PendingAuthChallenge};
use crate::infrastructure::memory_repo::InMemoryProposalRepository;
use crate::infrastructure::signer_set_repo::InMemorySignerSetRepository;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<RwLock<InMemoryProposalRepository>>,
    pub signer_set_repo: Arc<InMemorySignerSetRepository>,
    pub challenges: Arc<RwLock<HashMap<String, PendingAuthChallenge>>>,
    pub sessions: Arc<RwLock<HashMap<String, AuthSession>>>,
    pub auth_challenge_ttl_ms: u64,
    pub auth_session_ttl_ms: u64,
}

impl AppState {
    pub fn new(auth_challenge_ttl_ms: u64, auth_session_ttl_ms: u64) -> Self {
        Self {
            repo: Arc::new(RwLock::new(InMemoryProposalRepository::new())),
            signer_set_repo: Arc::new(InMemorySignerSetRepository::new()),
            challenges: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            auth_challenge_ttl_ms,
            auth_session_ttl_ms,
        }
    }
}
