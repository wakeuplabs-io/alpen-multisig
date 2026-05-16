//! Multisig authority (role) — client-side domain enum.
//!
//! Wire format uses snake_case strings matching the orchestrator HTTP contract.

use serde::{Deserialize, Serialize};

/// A multisig authority (governance role).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Authority {
    AlpenAdmin,
    StrataAdmin,
    SequencerManager,
    SecurityCouncil,
    PayoutAdmin,
}

/// Failure to parse a wire-format authority string.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthorityParseError {
    #[error("unknown authority: {0}")]
    Unknown(String),
}

impl Authority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Authority::AlpenAdmin => "alpen_admin",
            Authority::StrataAdmin => "strata_admin",
            Authority::SequencerManager => "sequencer_manager",
            Authority::SecurityCouncil => "security_council",
            Authority::PayoutAdmin => "payout_admin",
        }
    }

    pub fn from_wire(s: &str) -> Result<Self, AuthorityParseError> {
        match s {
            "alpen_admin" => Ok(Authority::AlpenAdmin),
            "strata_admin" => Ok(Authority::StrataAdmin),
            "sequencer_manager" => Ok(Authority::SequencerManager),
            "security_council" => Ok(Authority::SecurityCouncil),
            "payout_admin" => Ok(Authority::PayoutAdmin),
            other => Err(AuthorityParseError::Unknown(other.to_string())),
        }
    }
}

impl Serialize for Authority {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Authority {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Authority::from_wire(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_roundtrip_all_five_authorities() {
        for authority in [
            Authority::AlpenAdmin,
            Authority::StrataAdmin,
            Authority::SequencerManager,
            Authority::SecurityCouncil,
            Authority::PayoutAdmin,
        ] {
            let json = serde_json::to_string(&authority).unwrap();
            let parsed: Authority = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, authority);
        }
    }

    #[test]
    fn unknown_authority_fails_deserialize() {
        let result: Result<Authority, _> = serde_json::from_str("\"unknown\"");
        assert!(result.is_err());
    }
}
