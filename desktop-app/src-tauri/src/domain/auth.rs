use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strata_asm_params::Role;

use crate::domain::authority::Authority;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthRole {
    StrataAdministrator,
    StrataSequencerManager,
    AlpenAdministrator,
    StrataSecurityCouncil,
}

impl AuthRole {
    /// The ASM role that signs for `authority`, or `None` for the one authority upstream has no
    /// role for.
    ///
    /// Listed exhaustively rather than caught by `_`: a catch-all is how the council once sat in
    /// an error arm long after the orchestrator had mapped it, and the next authority added should
    /// be a compile error rather than a broadcast that fails at the last step.
    pub fn for_authority(authority: Authority) -> Option<Self> {
        match authority {
            Authority::StrataAdmin => Some(AuthRole::StrataAdministrator),
            Authority::SequencerManager => Some(AuthRole::StrataSequencerManager),
            Authority::AlpenAdmin => Some(AuthRole::AlpenAdministrator),
            Authority::SecurityCouncil => Some(AuthRole::StrataSecurityCouncil),
            Authority::PayoutAdmin => None,
        }
    }

    /// [`Self::for_authority`], with the error every ASM read reports for an unmapped authority.
    pub fn try_for_authority(authority: Authority) -> Result<Self, String> {
        Self::for_authority(authority).ok_or_else(|| {
            format!("authority `{authority:?}` is not mapped to ASM role authorization yet")
        })
    }

    pub fn to_upstream_role(self) -> Role {
        match self {
            AuthRole::StrataAdministrator => Role::StrataAdministrator,
            AuthRole::StrataSequencerManager => Role::StrataSequencerManager,
            AuthRole::AlpenAdministrator => Role::AlpenAdministrator,
            AuthRole::StrataSecurityCouncil => Role::StrataSecurityCouncil,
        }
    }

    pub fn as_wire_str(self) -> &'static str {
        match self {
            AuthRole::StrataAdministrator => "strata_administrator",
            AuthRole::StrataSequencerManager => "strata_sequencer_manager",
            AuthRole::AlpenAdministrator => "alpen_administrator",
            AuthRole::StrataSecurityCouncil => "strata_security_council",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthChallenge {
    pub challenge_id: String,
    pub challenge_hex: String,
    pub challenge_message: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Broadcast reads the signer set through this mapping, so every authority with an ASM role
    /// must resolve to exactly its own.
    #[test]
    fn payout_admin_is_the_only_unmapped_authority() {
        for (authority, role) in [
            (Authority::StrataAdmin, Role::StrataAdministrator),
            (Authority::SequencerManager, Role::StrataSequencerManager),
            (Authority::AlpenAdmin, Role::AlpenAdministrator),
            (Authority::SecurityCouncil, Role::StrataSecurityCouncil),
        ] {
            assert_eq!(
                AuthRole::try_for_authority(authority).map(AuthRole::to_upstream_role),
                Ok(role),
                "{authority:?}"
            );
        }
        assert!(AuthRole::try_for_authority(Authority::PayoutAdmin).is_err());
    }
}
