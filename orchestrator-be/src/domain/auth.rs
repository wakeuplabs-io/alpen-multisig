use crate::domain::authority::Authority;

#[derive(Debug, Clone)]
pub struct PendingAuthChallenge {
    pub authority: Authority,
    pub challenge_hex: String,
    pub challenge_message: String,
    pub expires_at_unix_ms: u64,
    pub consumed: bool,
}

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub authority: Authority,
    pub signer_pubkey: String,
    pub expires_at_unix_ms: u64,
}
