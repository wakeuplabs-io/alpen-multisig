/// Dust + protocol minimum sats locked in the commit output (SPS-51).
pub const COMMIT_DUST_SATS: u64 = 1500;

/// Estimated vbytes for an SPS-51 reveal transaction (conservative upper bound).
pub const REVEAL_TX_VBYTES: u64 = 350;

/// Conservative vsize estimate for the BDK commit tx (P2TR input + commit P2TR output
/// + P2TR change). Display-only — BDK computes the real fee when building.
pub const COMMIT_TX_VBYTES_ESTIMATE: u64 = 160;
