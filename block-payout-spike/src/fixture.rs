//! Mocked bridge params and software admin keys for the spike.
//!
//! In production these come from `params.toml` `[keys.admin]`, the operator N/N
//! aggregate, and the graph's `unstaking_image`. None of that is wired into
//! `alpen-multisig` yet.

use bitcoin::{
    hashes::{sha256, Hash},
    Network,
};
use secp256k1::{rand::rngs::OsRng, Keypair, SECP256K1};
use strata_bridge_connectors::prelude::ClaimPayoutConnector;

use crate::sign::SoftwareAdminSigner;

/// Size of the mocked admin set.
pub const ADMIN_COUNT: usize = 3;
/// Spend threshold used by the AdminBurn leaf.
pub const ADMIN_THRESHOLD: usize = 2;

/// Software-key stand-in for a Payout Administrator quorum and the connector they spend.
#[derive(Debug, Clone)]
pub struct SpikeFixture {
    /// Operator N/N key used as the Taproot internal key (key-path belongs to operators).
    pub n_of_n_keypair: Keypair,
    /// Independent admin signers, indexed in `params.toml` order.
    pub admins: Vec<SoftwareAdminSigner>,
    /// Preimage whose SHA-256 is the unstaking leaf image (unused by AdminBurn).
    pub unstaking_preimage: [u8; 32],
    /// Connector matching the mocked params.
    pub connector: ClaimPayoutConnector,
}

impl SpikeFixture {
    /// Generate a fresh 2-of-3 fixture on regtest.
    pub fn new() -> Self {
        let n_of_n_keypair = Keypair::new(SECP256K1, &mut OsRng);
        let admin_keypairs: Vec<Keypair> = (0..ADMIN_COUNT)
            .map(|_| Keypair::new(SECP256K1, &mut OsRng))
            .collect();
        let unstaking_preimage: [u8; 32] = secp256k1::rand::random();

        let connector = ClaimPayoutConnector::new(
            Network::Regtest,
            n_of_n_keypair.x_only_public_key().0,
            admin_keypairs
                .iter()
                .map(|keypair| keypair.x_only_public_key().0)
                .collect(),
            ADMIN_THRESHOLD,
            sha256::Hash::hash(&unstaking_preimage),
        );

        let admins = admin_keypairs
            .into_iter()
            .enumerate()
            .map(|(pubkey_index, keypair)| SoftwareAdminSigner::new(pubkey_index, keypair))
            .collect();

        Self {
            n_of_n_keypair,
            admins,
            unstaking_preimage,
            connector,
        }
    }

    /// Signer at `params.toml` index. Panics if out of range.
    pub fn signer(&self, pubkey_index: usize) -> &SoftwareAdminSigner {
        &self.admins[pubkey_index]
    }
}

impl Default for SpikeFixture {
    fn default() -> Self {
        Self::new()
    }
}
