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

/// A public key identifying a signer within an authority's signer set.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SignerPubkey(pub String);

/// The canonical signer set for a given authority, derived from onchain ASM state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerSet {
    pub authority: Authority,
    pub signers: Vec<SignerPubkey>,
    pub threshold: u32,
}
