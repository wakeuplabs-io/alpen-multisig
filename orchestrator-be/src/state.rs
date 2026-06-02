use crate::application::traits::ProposalRepository;
use crate::domain::auth::{AuthSession, PendingAuthChallenge};
use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<dyn ProposalRepository>,
    pub asm_rpc_url: Arc<String>,
    pub challenges: Arc<RwLock<HashMap<String, PendingAuthChallenge>>>,
    pub sessions: Arc<RwLock<HashMap<String, AuthSession>>>,
    pub auth_challenge_ttl_ms: u64,
    pub auth_session_ttl_ms: u64,
    /// Used by `/ready` to verify Bitcoin RPC reachability (coordination service does not broadcast).
    pub btc_client: Arc<dyn BitcoinRpcClient>,
}

impl AppState {
    pub fn new(
        repo: Arc<dyn ProposalRepository>,
        asm_rpc_url: String,
        auth_challenge_ttl_ms: u64,
        auth_session_ttl_ms: u64,
        btc_client: Arc<dyn BitcoinRpcClient>,
    ) -> Self {
        Self {
            repo,
            asm_rpc_url: Arc::new(asm_rpc_url),
            challenges: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            auth_challenge_ttl_ms,
            auth_session_ttl_ms,
            btc_client,
        }
    }
}
