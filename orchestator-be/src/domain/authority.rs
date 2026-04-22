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
