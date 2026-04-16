//! Multisig authority (role) — client-side domain enum.
//!
//! An `Authority` identifies which multisig group is acting. It is a pure value type:
//! it does **not** carry signer set, threshold, or `last_seqno` — those live onchain
//! and are derived by the backend from the ASM state (see PRD §3 Authority Isolation).
//!
//! The wire format uses snake_case strings (`"strata_admin"`) to match the orchestrator
//! HTTP contract.

use serde::{Deserialize, Serialize};

/// A multisig authority (governance role).
///
/// Single variant for POC-4; new roles will be added as the feature set grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Authority {
    StrataAdmin,
}

/// Failure to parse a wire-format authority string.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AuthorityParseError {
    #[error("unknown authority: {0}")]
    Unknown(String),
}

impl Authority {
    /// Returns the canonical wire string used by the orchestrator HTTP contract.
    pub fn as_str(&self) -> &'static str {
        match self {
            Authority::StrataAdmin => "strata_admin",
        }
    }

    /// Parses a wire-format authority string.
    pub fn from_wire(s: &str) -> Result<Self, AuthorityParseError> {
        match s {
            "strata_admin" => Ok(Authority::StrataAdmin),
            other => Err(AuthorityParseError::Unknown(other.to_string())),
        }
    }
}

// ─── Serde (wire format) ────────────────────────────────────────────────────

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

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strata_admin_as_str() {
        assert_eq!(Authority::StrataAdmin.as_str(), "strata_admin");
    }

    #[test]
    fn test_wire_roundtrip() {
        let parsed = Authority::from_wire(Authority::StrataAdmin.as_str())
            .expect("wire roundtrip must succeed");
        assert_eq!(parsed, Authority::StrataAdmin);
    }

    #[test]
    fn test_unknown_authority_errors() {
        let err = Authority::from_wire("unknown").unwrap_err();
        assert_eq!(err, AuthorityParseError::Unknown("unknown".to_string()));
    }

    #[test]
    fn test_serde_serialize() {
        let json = serde_json::to_string(&Authority::StrataAdmin).unwrap();
        assert_eq!(json, "\"strata_admin\"");
    }

    #[test]
    fn test_serde_deserialize() {
        let parsed: Authority = serde_json::from_str("\"strata_admin\"").unwrap();
        assert_eq!(parsed, Authority::StrataAdmin);
    }

    #[test]
    fn test_serde_deserialize_unknown_fails() {
        let result: Result<Authority, _> = serde_json::from_str("\"unknown\"");
        assert!(result.is_err());
    }
}
