//! Signer sets for in-process ASM params.
//!
//! The admin params name each role's signers by P2WPKH address, and the ASM refuses an address
//! whose network differs from the anchor's. The harness anchors on regtest.

use std::num::NonZero;

use bitcoin::Network;
use strata_asm_admin_threshold_sig::P2wpkhAddress;
use strata_asm_admin_types::UncheckedThresholdConfig;

/// A role's genesis signer set, written the way a regtest params file writes it.
pub fn regtest_signer_config(
    signers: &[P2wpkhAddress],
    threshold: NonZero<u8>,
) -> UncheckedThresholdConfig {
    let addresses = signers
        .iter()
        .map(|signer| signer.to_address(Network::Regtest).into_unchecked())
        .collect();
    UncheckedThresholdConfig::try_new(addresses, threshold).expect("valid signer set")
}
