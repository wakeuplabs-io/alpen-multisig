use serde::{Deserialize, Serialize};

/// Challenge issued by the backend for the auth handshake.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub nonce: String,
    pub expires_at: String,
}

/// Full session as returned by the backend (includes session_id used as Bearer token).
#[derive(Debug, Deserialize)]
pub struct BackendSession {
    pub session_id: String,
    pub signer_pubkey: String,
    pub authority: String,
    pub expires_at: String,
}

/// Session info exposed to the frontend (no session_id — kept only in AppState).
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub signer_pubkey: String,
    pub authority: String,
    pub expires_at: String,
}

/// Payload sent by the frontend to create a session.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionPayload {
    pub ephemeral_pubkey: String,
    pub nonce: String,
    pub attestation_signature: String,
    pub signer_pubkey: String,
    pub authority: String,
}
