//! Client-side session domain types — pure data, no framework dependencies.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub nonce: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
pub struct BackendSession {
    pub session_id: String,
    pub signer_pubkey: String,
    pub authority: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub signer_pubkey: String,
    pub authority: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionPayload {
    pub ephemeral_pubkey: String,
    pub nonce: String,
    pub attestation_signature: String,
    pub signer_pubkey: String,
    pub authority: String,
}
