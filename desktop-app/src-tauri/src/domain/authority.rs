/// The five multisig governance authorities defined in SPS-50/SPS-65.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authority {
    AlpenAdmin,
    StrataAdmin,
    SequencerManager,
    SecurityCouncil,
    PayoutAdmin,
}
