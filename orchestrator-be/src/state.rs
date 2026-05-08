use bitcoin::key::UntweakedKeypair;
use bitcoin::Network;
use strata_l1_txfmt::MagicBytes;

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
    pub btc_client: Arc<dyn BitcoinRpcClient>,
    pub operator_keypair: Arc<UntweakedKeypair>,
    pub confirm_poll_interval_ms: u64,
    pub confirm_timeout_ms: u64,
    pub bitcoin_magic_bytes: MagicBytes,
    pub bitcoin_network: Network,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repo: Arc<dyn ProposalRepository>,
        asm_rpc_url: String,
        auth_challenge_ttl_ms: u64,
        auth_session_ttl_ms: u64,
        btc_client: Arc<dyn BitcoinRpcClient>,
        operator_keypair: UntweakedKeypair,
        confirm_poll_interval_ms: u64,
        confirm_timeout_ms: u64,
        bitcoin_magic_bytes: MagicBytes,
        bitcoin_network: Network,
    ) -> Self {
        Self {
            repo,
            asm_rpc_url: Arc::new(asm_rpc_url),
            challenges: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            auth_challenge_ttl_ms,
            auth_session_ttl_ms,
            btc_client,
            operator_keypair: Arc::new(operator_keypair),
            confirm_poll_interval_ms,
            confirm_timeout_ms,
            bitcoin_magic_bytes,
            bitcoin_network,
        }
    }
}
