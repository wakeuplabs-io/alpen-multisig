//! Feasibility spike: construct and software-sign a `block_payout` (`AdminBurnTx`)
//! as a K-of-M Payout Administrator set, without a hardware wallet.
//!
//! This crate is its own Cargo workspace so it can path-depend on `strata-bridge`
//! crates without forcing `bitcoin 0.32.9` onto the parent `alpen-multisig` workspace.

pub mod build;
pub mod fixture;
pub mod from_bridge;
pub mod sign;

pub use build::{build_admin_burn, dummy_fee_input, FEE};
pub use fixture::{SpikeFixture, ADMIN_COUNT, ADMIN_THRESHOLD};
pub use sign::{collect_threshold, SoftwareAdminSigner};
