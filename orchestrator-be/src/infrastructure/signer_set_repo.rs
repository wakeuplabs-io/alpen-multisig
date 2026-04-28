//! In-memory signer set repository used for authorization checks.

use crate::application::traits::SignerSetRepository;
use crate::domain::authority::Authority;
use crate::error::AppError;
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use std::collections::{HashMap, HashSet};

pub(crate) struct InMemorySignerSetRepository {
    memberships: HashMap<Authority, HashSet<String>>,
}

impl InMemorySignerSetRepository {
    pub(crate) fn new() -> Self {
        // Deterministic fixture key for local/dev/test authorization:
        // sk=1 -> 0279be...f81798, sk=2 -> 02c604...09ee5.
        let signer_a =
            "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798".to_string();
        let signer_b_fixture =
            "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5".to_string();
        let signer_b_from_sk2 = {
            let mut sk_bytes = [0u8; 32];
            sk_bytes[31] = 2;
            let sk = SecretKey::from_slice(&sk_bytes).expect("valid deterministic fixture key");
            let pk = PublicKey::from_secret_key(&Secp256k1::new(), &sk);
            hex::encode(pk.serialize())
        };

        let mut memberships: HashMap<Authority, HashSet<String>> = HashMap::new();
        memberships.insert(
            Authority::StrataAdmin,
            HashSet::from([signer_a, signer_b_fixture, signer_b_from_sk2]),
        );
        memberships.insert(Authority::SequencerManager, HashSet::new());
        memberships.insert(Authority::SecurityCouncil, HashSet::new());
        memberships.insert(Authority::AlpenAdmin, HashSet::new());
        memberships.insert(Authority::PayoutAdmin, HashSet::new());

        Self { memberships }
    }
}

impl SignerSetRepository for InMemorySignerSetRepository {
    fn is_signer_for_authority(
        &self,
        authority: Authority,
        signer_pubkey: &str,
    ) -> Result<bool, AppError> {
        let needle = signer_pubkey.to_ascii_lowercase();
        let is_member = self
            .memberships
            .get(&authority)
            .is_some_and(|signers| signers.contains(&needle));
        Ok(is_member)
    }
}
