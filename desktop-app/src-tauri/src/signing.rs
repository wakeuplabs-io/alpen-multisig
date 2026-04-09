use std::num::NonZero;

use bitcoin::secp256k1::{ecdsa::Signature, Message, PublicKey, SecretKey, SECP256K1};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use strata_asm_params::Role;
use strata_asm_txs_admin::actions::updates::multisig::MultisigUpdate;
use strata_asm_txs_admin::actions::{MultisigAction, Sighash, UpdateAction};
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::{ThresholdConfig, ThresholdConfigUpdate};

/// Keypair representation for IPC (private key included for demo only — never in production)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoKeypair {
    pub secret_key_hex: String,
    pub public_key_hex: String,
}

/// Result of key generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateKeysResult {
    pub keypairs: Vec<DemoKeypair>,
    pub threshold: u32,
    pub num_signers: u32,
}

/// Result of sighash computation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SighashResult {
    pub sighash_hex: String,
    pub seqno: u64,
    pub action_description: String,
}

/// A collected signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureResult {
    pub signer_index: u32,
    pub public_key_hex: String,
    pub signature_hex: String,
}

/// Verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub valid: bool,
    pub signatures_verified: u32,
    pub threshold_required: u32,
}

/// Build a deterministic demo action: Strata Admin signer set update.
/// Uses a fixed key so that sighash output is reproducible for the same seqno.
fn build_demo_action() -> MultisigAction {
    // Fixed 32-byte secret for deterministic demo key (not used for signing)
    let demo_bytes = [0x42u8; 32];
    let demo_sk = SecretKey::from_slice(&demo_bytes).expect("valid fixed key");
    let new_signer = CompressedPublicKey::from(PublicKey::from_secret_key(SECP256K1, &demo_sk));

    let config_update =
        ThresholdConfigUpdate::new(vec![new_signer], vec![], NonZero::new(2).expect("non-zero"));
    let multisig_update = MultisigUpdate::new(config_update, Role::StrataAdministrator);
    MultisigAction::Update(UpdateAction::Multisig(multisig_update))
}

#[tauri::command]
pub fn generate_demo_keys(num_signers: u32, threshold: u32) -> Result<GenerateKeysResult, String> {
    if threshold == 0 || threshold > num_signers {
        return Err(format!(
            "invalid threshold: {} of {} signers",
            threshold, num_signers
        ));
    }

    let keypairs: Vec<DemoKeypair> = (0..num_signers)
        .map(|_| {
            let sk = SecretKey::new(&mut OsRng);
            let pk = PublicKey::from_secret_key(SECP256K1, &sk);
            DemoKeypair {
                secret_key_hex: hex::encode(sk.secret_bytes()),
                public_key_hex: hex::encode(pk.serialize()),
            }
        })
        .collect();

    Ok(GenerateKeysResult {
        keypairs,
        threshold,
        num_signers,
    })
}

#[tauri::command]
pub fn compute_sighash(seqno: u64) -> Result<SighashResult, String> {
    let action = build_demo_action();
    let sighash = action.compute_sighash(seqno);

    Ok(SighashResult {
        sighash_hex: hex::encode(sighash.0),
        seqno,
        action_description: "Strata Admin signer set update (add 1 key)".to_string(),
    })
}

#[tauri::command]
pub fn sign_sighash(
    secret_key_hex: String,
    sighash_hex: String,
) -> Result<SignatureResult, String> {
    let sk_bytes =
        hex::decode(&secret_key_hex).map_err(|e| format!("invalid secret key hex: {e}"))?;
    let sk = SecretKey::from_slice(&sk_bytes).map_err(|e| format!("invalid secret key: {e}"))?;

    let hash_bytes = hex::decode(&sighash_hex).map_err(|e| format!("invalid sighash hex: {e}"))?;
    let msg = Message::from_digest_slice(&hash_bytes)
        .map_err(|e| format!("invalid sighash (must be 32 bytes): {e}"))?;

    let sig = SECP256K1.sign_ecdsa(&msg, &sk);
    let pk = PublicKey::from_secret_key(SECP256K1, &sk);

    Ok(SignatureResult {
        signer_index: 0, // caller tracks index
        public_key_hex: hex::encode(pk.serialize()),
        signature_hex: hex::encode(sig.serialize_compact()),
    })
}

#[tauri::command]
pub fn verify_threshold(
    public_keys_hex: Vec<String>,
    threshold: u32,
    signatures_hex: Vec<String>,
    sighash_hex: String,
) -> Result<VerifyResult, String> {
    let hash_bytes = hex::decode(&sighash_hex).map_err(|e| format!("invalid sighash hex: {e}"))?;
    let msg =
        Message::from_digest_slice(&hash_bytes).map_err(|e| format!("invalid sighash: {e}"))?;

    // Parse public keys into CompressedPublicKey for ThresholdConfig
    let pub_keys: Vec<CompressedPublicKey> = public_keys_hex
        .iter()
        .map(|h| {
            let bytes = hex::decode(h).map_err(|e| format!("invalid public key hex: {e}"))?;
            let pk =
                PublicKey::from_slice(&bytes).map_err(|e| format!("invalid public key: {e}"))?;
            Ok(CompressedPublicKey::from(pk))
        })
        .collect::<Result<Vec<_>, String>>()?;

    // Validate threshold config (ensures keys/threshold are coherent)
    let threshold_nz = NonZero::new(threshold as u8).ok_or("threshold must be > 0")?;
    ThresholdConfig::try_new(pub_keys, threshold_nz)
        .map_err(|e| format!("invalid threshold config: {e}"))?;

    // Parse and verify each signature, counting valid ones
    let mut valid_count: u32 = 0;
    for sig_hex in &signatures_hex {
        let sig_bytes = hex::decode(sig_hex).map_err(|e| format!("invalid signature hex: {e}"))?;
        let sig =
            Signature::from_compact(&sig_bytes).map_err(|e| format!("invalid signature: {e}"))?;

        // Check if this signature is valid against any signer in the config
        for pk_hex in &public_keys_hex {
            let pk_bytes =
                hex::decode(pk_hex).map_err(|e| format!("invalid public key hex: {e}"))?;
            let pk =
                PublicKey::from_slice(&pk_bytes).map_err(|e| format!("invalid public key: {e}"))?;
            if SECP256K1.verify_ecdsa(&msg, &sig, &pk).is_ok() {
                valid_count += 1;
                break;
            }
        }
    }

    Ok(VerifyResult {
        valid: valid_count >= threshold,
        signatures_verified: valid_count,
        threshold_required: threshold,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_signing_flow_2_of_3() {
        let keys = generate_demo_keys(3, 2).expect("key generation should succeed");
        assert_eq!(keys.keypairs.len(), 3);
        assert_eq!(keys.threshold, 2);
        assert_eq!(keys.num_signers, 3);

        let sighash = compute_sighash(1).expect("sighash computation should succeed");
        assert_eq!(sighash.seqno, 1);
        assert!(!sighash.sighash_hex.is_empty());
        assert_eq!(sighash.sighash_hex.len(), 64);

        let sig0 = sign_sighash(
            keys.keypairs[0].secret_key_hex.clone(),
            sighash.sighash_hex.clone(),
        )
        .expect("signing should succeed");
        assert!(!sig0.signature_hex.is_empty());

        let sig2 = sign_sighash(
            keys.keypairs[2].secret_key_hex.clone(),
            sighash.sighash_hex.clone(),
        )
        .expect("signing should succeed");
        assert!(!sig2.signature_hex.is_empty());

        let pub_keys: Vec<String> = keys
            .keypairs
            .iter()
            .map(|k| k.public_key_hex.clone())
            .collect();
        let result = verify_threshold(
            pub_keys,
            2,
            vec![sig0.signature_hex, sig2.signature_hex],
            sighash.sighash_hex,
        )
        .expect("verification should succeed");

        assert!(result.valid);
        assert_eq!(result.signatures_verified, 2);
        assert_eq!(result.threshold_required, 2);
    }

    #[test]
    fn test_below_threshold_fails() {
        let keys = generate_demo_keys(3, 2).expect("key generation should succeed");
        let sighash = compute_sighash(1).expect("sighash computation should succeed");

        let sig0 = sign_sighash(
            keys.keypairs[0].secret_key_hex.clone(),
            sighash.sighash_hex.clone(),
        )
        .expect("signing should succeed");

        let pub_keys: Vec<String> = keys
            .keypairs
            .iter()
            .map(|k| k.public_key_hex.clone())
            .collect();
        let result = verify_threshold(pub_keys, 2, vec![sig0.signature_hex], sighash.sighash_hex)
            .expect("verification call should succeed");

        assert!(!result.valid);
        assert_eq!(result.signatures_verified, 1);
        assert_eq!(result.threshold_required, 2);
    }

    #[test]
    fn test_sighash_deterministic() {
        let hash1 = compute_sighash(1).expect("should succeed");
        let hash2 = compute_sighash(1).expect("should succeed");
        assert_eq!(hash1.sighash_hex, hash2.sighash_hex);
    }

    #[test]
    fn test_different_seqno_different_sighash() {
        let hash1 = compute_sighash(1).expect("should succeed");
        let hash2 = compute_sighash(2).expect("should succeed");
        assert_ne!(hash1.sighash_hex, hash2.sighash_hex);
    }

    #[test]
    fn test_invalid_secret_key_hex() {
        let result = sign_sighash("not_valid_hex".to_string(), "aa".repeat(32));
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_sighash_hex() {
        let keys = generate_demo_keys(1, 1).expect("key generation should succeed");
        let result = sign_sighash(
            keys.keypairs[0].secret_key_hex.clone(),
            "tooshort".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_signatures_fails() {
        let keys = generate_demo_keys(3, 2).expect("key generation should succeed");
        let sighash = compute_sighash(1).expect("should succeed");

        let pub_keys: Vec<String> = keys
            .keypairs
            .iter()
            .map(|k| k.public_key_hex.clone())
            .collect();
        let result = verify_threshold(pub_keys, 2, vec![], sighash.sighash_hex)
            .expect("verification call should succeed");

        assert!(!result.valid);
        assert_eq!(result.signatures_verified, 0);
    }
}
