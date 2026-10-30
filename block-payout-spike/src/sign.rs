//! Independent software signers for the AdminBurn leaf.
//!
//! Each signer holds only its own key and produces one `AdminSignature`. The
//! coordinator collects K of them; it does not re-use a helper that signs the
//! whole threshold in one shot.

use secp256k1::Keypair;
use strata_bridge_connectors::{claim_payout::AdminSignature, SigningInfo};

/// One Payout Administrator with a software (non-hardware) key.
#[derive(Debug, Clone)]
pub struct SoftwareAdminSigner {
    /// Index of this key in the bridge `[keys.admin]` list. Order is consensus-critical.
    pub pubkey_index: usize,
    keypair: Keypair,
}

impl SoftwareAdminSigner {
    /// Wrap an existing keypair at a known admin index.
    pub fn new(pubkey_index: usize, keypair: Keypair) -> Self {
        Self {
            pubkey_index,
            keypair,
        }
    }

    /// BIP-340 script-path signature over the AdminBurn sighash. Untweaked admin key.
    pub fn sign(&self, signing_info: SigningInfo) -> AdminSignature {
        AdminSignature {
            pubkey_index: self.pubkey_index,
            signature: signing_info.sign(&self.keypair),
        }
    }
}

/// Keep the first `threshold` signatures. Dedup/index checks happen in the connector.
pub fn collect_threshold(
    signatures: impl IntoIterator<Item = AdminSignature>,
    threshold: usize,
) -> Vec<AdminSignature> {
    let mut collected: Vec<AdminSignature> = signatures.into_iter().collect();
    collected.truncate(threshold);
    collected
}
