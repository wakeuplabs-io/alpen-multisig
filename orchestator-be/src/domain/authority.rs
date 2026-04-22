use serde::{Deserialize, Serialize};

/// The five multisig governance authorities defined by the protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Authority {
    AlpenAdmin,
    StrataAdmin,
    SequencerManager,
    SecurityCouncil,
    PayoutAdmin,
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

    pub fn from_wire(s: &str) -> Result<Self, String> {
        match s {
            "alpen_admin" => Ok(Authority::AlpenAdmin),
            "strata_admin" => Ok(Authority::StrataAdmin),
            "sequencer_manager" => Ok(Authority::SequencerManager),
            "security_council" => Ok(Authority::SecurityCouncil),
            "payout_admin" => Ok(Authority::PayoutAdmin),
            other => Err(format!("unknown authority: {other}")),
        }
    }
}

/// A public key identifying a signer within an authority's signer set.
#[allow(dead_code)] // Planned: signer verification against ASM state
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignerPubkey(pub String);

/// The canonical signer set for a given authority, derived from onchain ASM state.
#[allow(dead_code)] // Planned: signer verification against ASM state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerSet {
    pub authority: Authority,
    pub signers: Vec<SignerPubkey>,
    pub threshold: u32,
}
