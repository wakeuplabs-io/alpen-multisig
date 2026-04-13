//! Auth business logic — challenge issuance and session lifecycle.
//!
//! Stubs to be implemented when the signer attestation flow lands.

use crate::domain::session::AuthChallenge;
use crate::error::Result;

/// Input for creating a new session after attestation verification.
#[allow(dead_code)] // Planned: auth flow implementation
pub(crate) struct CreateSessionInput {
    pub(crate) ephemeral_pubkey: String,
    pub(crate) nonce: String,
    pub(crate) attestation_signature: String,
    pub(crate) signer_pubkey: String,
    pub(crate) authority: String,
}

/// Result of creating a new session.
#[allow(dead_code)] // Planned: auth flow implementation
pub(crate) struct SessionResult {
    pub(crate) session_id: String,
    pub(crate) expires_at: String,
}

pub(crate) fn get_challenge(_pubkey: &str, _authority: &str) -> Result<AuthChallenge> {
    todo!("issue nonce challenge for signer")
}

pub(crate) fn create_session(_input: CreateSessionInput) -> Result<SessionResult> {
    todo!("verify attestation against ASM signer set, create session")
}

pub(crate) fn delete_session(_session_id: &str) -> Result<()> {
    todo!("invalidate session")
}
