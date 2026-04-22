use crate::domain::authority::Authority;

#[derive(Debug, Clone)]
#[allow(dead_code)] // token field redundantly stored alongside HashMap key; kept for session context
pub struct Session {
    pub token: String,
    pub authority: Authority,
    pub signer_pubkey: String,
    pub ephemeral_pubkey: String,
    pub expires_at: i64,
}
