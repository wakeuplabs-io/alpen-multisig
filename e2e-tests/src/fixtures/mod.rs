//! Deterministic test fixtures for integration tests.
//!
//! Fixtures use only `const` data and `&'static str` JSON — no network I/O and no
//! reading arbitrary paths from disk (avoids "works on my machine" drift). Optional
//! future profiles can gate file-based fixtures behind an explicit env var.

mod signer_update_enacted;

pub use signer_update_enacted::{
    administration_init_config, assert_mnemonic_matches_strata_admin_keys,
    decode_administration_subproto, parse_admin_section, strata_admin_confirmation_depth,
    strata_admin_keys_hex, SignerUpdateEnactedFixture, DEFAULT_REPO_ASM, FAST_ENACTMENT,
};
