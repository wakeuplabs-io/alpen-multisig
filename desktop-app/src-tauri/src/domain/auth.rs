use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strata_asm_params::Role;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthRole {
    StrataAdministrator,
    StrataSequencerManager,
    AlpenAdministrator,
}

impl AuthRole {
    pub fn to_upstream_role(self) -> Role {
        match self {
            AuthRole::StrataAdministrator => Role::StrataAdministrator,
            AuthRole::StrataSequencerManager => Role::StrataSequencerManager,
            AuthRole::AlpenAdministrator => Role::AlpenAdministrator,
        }
    }

    pub fn as_wire_str(self) -> &'static str {
        match self {
            AuthRole::StrataAdministrator => "strata_administrator",
            AuthRole::StrataSequencerManager => "strata_sequencer_manager",
            AuthRole::AlpenAdministrator => "alpen_administrator",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthChallenge {
    pub challenge_id: String,
    pub challenge_hex: String,
    pub nonce_hex: String,
    pub domain: String,
    pub role: AuthRole,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSession {
    pub role: AuthRole,
    pub signer_pubkey_hex: String,
    pub authenticated_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub membership_fetched_at_unix_ms: u64,
}

#[derive(Debug, Clone)]
pub struct PendingChallenge {
    pub challenge: AuthChallenge,
    pub consumed: bool,
}

#[derive(Debug, Clone)]
pub struct MembershipCache {
    pub fetched_at_unix_ms: u64,
    pub role_to_keys: HashMap<AuthRole, Vec<String>>,
}
