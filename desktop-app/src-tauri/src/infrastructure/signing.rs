use std::num::NonZero;
use std::str::FromStr;

use bip39::Mnemonic;
use bitcoin::address::KnownHrp;
use bitcoin::bip32::{DerivationPath, Xpriv};
use bitcoin::secp256k1::{ecdsa::Signature, Message, PublicKey, SecretKey, SECP256K1};
use bitcoin::{key::TweakedPublicKey, secp256k1::XOnlyPublicKey};
use ssz::Decode;
use strata_asm_txs_admin::actions::{MultisigAction, Sighash};
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::ThresholdConfig;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SighashResult {
    pub sighash_hex: String,
    pub seqno: u64,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureResult {
    pub public_key_hex: String,
    pub signature_hex: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MnemonicAddressEntry {
    pub index: u32,
    pub derivation_path: String,
    pub address: String,
    pub public_key_hex: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResult {
    pub valid: bool,
    pub signatures_verified: u32,
    pub threshold_required: u32,
}

// ---------------------------------------------------------------------------
// Signing operations
// ---------------------------------------------------------------------------

/// Compute the SPS-65 tagged sighash for a given action and sequence number.
/// The action is passed as SSZ-serialized hex (MultisigAction uses SSZ, not serde).
pub fn compute_sighash(seqno: u64, action_hex: &str) -> Result<SighashResult, String> {
    let action_bytes = hex::decode(action_hex).map_err(|e| format!("invalid action hex: {e}"))?;
    let action = MultisigAction::from_ssz_bytes(&action_bytes)
        .map_err(|e| format!("invalid ssz-encoded action: {e:?}"))?;
    let sighash = action.compute_sighash(seqno);

    Ok(SighashResult {
        sighash_hex: hex::encode(sighash.0),
        seqno,
    })
}

/// Sign a sighash with an ECDSA private key. Returns the signature and corresponding public key.
pub fn sign_sighash(secret_key_hex: &str, sighash_hex: &str) -> Result<SignatureResult, String> {
    let sk_bytes =
        hex::decode(secret_key_hex).map_err(|e| format!("invalid secret key hex: {e}"))?;
    let sk = SecretKey::from_slice(&sk_bytes).map_err(|e| format!("invalid secret key: {e}"))?;

    let hash_bytes = hex::decode(sighash_hex).map_err(|e| format!("invalid sighash hex: {e}"))?;
    let msg = Message::from_digest_slice(&hash_bytes)
        .map_err(|e| format!("invalid sighash (must be 32 bytes): {e}"))?;

    let sig = SECP256K1.sign_ecdsa(&msg, &sk);
    let pk = PublicKey::from_secret_key(SECP256K1, &sk);

    Ok(SignatureResult {
        public_key_hex: hex::encode(pk.serialize()),
        signature_hex: hex::encode(sig.serialize_compact()),
    })
}

fn derive_secret_key_from_mnemonic_path(
    mnemonic: &str,
    passphrase: &str,
    derivation_path: &str,
) -> Result<SecretKey, String> {
    let normalized = mnemonic.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Err("mnemonic is required".to_string());
    }

    let mnemonic =
        Mnemonic::parse(&normalized).map_err(|e| format!("invalid BIP39 mnemonic: {e}"))?;
    let seed = mnemonic.to_seed(passphrase);
    let path = DerivationPath::from_str(derivation_path)
        .map_err(|e| format!("invalid derivation path: {e}"))?;

    let master = Xpriv::new_master(bitcoin::Network::Bitcoin, &seed)
        .map_err(|e| format!("failed to create BIP32 master key: {e}"))?;
    let derived = master
        .derive_priv(SECP256K1, &path)
        .map_err(|e| format!("failed to derive path {derivation_path}: {e}"))?;

    Ok(derived.private_key)
}

pub fn list_mnemonic_addresses(
    mnemonic: &str,
    passphrase: &str,
    count: u32,
) -> Result<Vec<MnemonicAddressEntry>, String> {
    let mut out = Vec::with_capacity(count as usize);
    for n in 0..count {
        let derivation_path = format!("m/86'/0'/73'/0/{n}");
        let secret_key =
            derive_secret_key_from_mnemonic_path(mnemonic, passphrase, &derivation_path)?;
        let pubkey = PublicKey::from_secret_key(SECP256K1, &secret_key);
        let xonly = XOnlyPublicKey::from(pubkey);
        let tweaked = TweakedPublicKey::dangerous_assume_tweaked(xonly);
        let address = bitcoin::Address::p2tr_tweaked(tweaked, KnownHrp::Mainnet);
        out.push(MnemonicAddressEntry {
            index: n,
            derivation_path,
            address: address.to_string(),
            public_key_hex: hex::encode(pubkey.serialize()),
        });
    }
    Ok(out)
}

pub fn sign_with_mnemonic_path(
    mnemonic: &str,
    passphrase: &str,
    derivation_path: &str,
    sighash_hex: &str,
) -> Result<SignatureResult, String> {
    let secret_key = derive_secret_key_from_mnemonic_path(mnemonic, passphrase, derivation_path)?;
    let hash_bytes = hex::decode(sighash_hex).map_err(|e| format!("invalid sighash hex: {e}"))?;
    let message = Message::from_digest_slice(&hash_bytes)
        .map_err(|e| format!("invalid sighash (must be 32 bytes): {e}"))?;
    let sig = SECP256K1.sign_ecdsa(&message, &secret_key);
    let public_key = PublicKey::from_secret_key(SECP256K1, &secret_key);

    Ok(SignatureResult {
        public_key_hex: hex::encode(public_key.serialize()),
        signature_hex: hex::encode(sig.serialize_compact()),
    })
}

/// Verify ECDSA signatures against a threshold signer configuration.
pub fn verify_threshold(
    public_keys_hex: &[String],
    threshold: u32,
    signatures_hex: &[String],
    sighash_hex: &str,
) -> Result<VerifyResult, String> {
    let hash_bytes = hex::decode(sighash_hex).map_err(|e| format!("invalid sighash hex: {e}"))?;
    let msg =
        Message::from_digest_slice(&hash_bytes).map_err(|e| format!("invalid sighash: {e}"))?;

    // Parse public keys and validate threshold config
    let pub_keys: Vec<CompressedPublicKey> = public_keys_hex
        .iter()
        .map(|h| {
            let bytes = hex::decode(h).map_err(|e| format!("invalid public key hex: {e}"))?;
            let pk =
                PublicKey::from_slice(&bytes).map_err(|e| format!("invalid public key: {e}"))?;
            Ok(CompressedPublicKey::from(pk))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let threshold_nz = NonZero::new(threshold as u8).ok_or("threshold must be > 0")?;
    ThresholdConfig::try_new(pub_keys, threshold_nz)
        .map_err(|e| format!("invalid threshold config: {e}"))?;

    // Verify each signature against the signer set
    let mut valid_count: u32 = 0;
    for sig_hex in signatures_hex {
        let sig_bytes = hex::decode(sig_hex).map_err(|e| format!("invalid signature hex: {e}"))?;
        let sig =
            Signature::from_compact(&sig_bytes).map_err(|e| format!("invalid signature: {e}"))?;

        for pk_hex in public_keys_hex {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;
    use ssz::Encode;
    use strata_asm_params::Role;
    use strata_asm_txs_admin::actions::updates::multisig::MultisigUpdate;
    use strata_asm_txs_admin::actions::UpdateAction;
    use strata_crypto::threshold_signature::ThresholdConfigUpdate;

    // -- Test helpers --------------------------------------------------------

    struct DemoKeypair {
        secret_key_hex: String,
        public_key_hex: String,
    }

    fn generate_demo_keys(num_signers: u32) -> Vec<DemoKeypair> {
        (0..num_signers)
            .map(|_| {
                let sk = SecretKey::new(&mut OsRng);
                let pk = PublicKey::from_secret_key(SECP256K1, &sk);
                DemoKeypair {
                    secret_key_hex: hex::encode(sk.secret_bytes()),
                    public_key_hex: hex::encode(pk.serialize()),
                }
            })
            .collect()
    }

    fn build_demo_action() -> MultisigAction {
        let demo_bytes = [0x42u8; 32];
        let demo_sk = SecretKey::from_slice(&demo_bytes).expect("valid fixed key");
        let new_signer = CompressedPublicKey::from(PublicKey::from_secret_key(SECP256K1, &demo_sk));
        let config_update = ThresholdConfigUpdate::new(
            vec![new_signer],
            vec![],
            NonZero::new(2).expect("non-zero"),
        );
        let multisig_update = MultisigUpdate::new(config_update, Role::StrataAdministrator);
        MultisigAction::Update(UpdateAction::Multisig(multisig_update))
    }

    fn demo_action_hex() -> String {
        hex::encode(build_demo_action().as_ssz_bytes())
    }

    // -- compute_sighash tests -----------------------------------------------

    #[test]
    fn test_compute_sighash_returns_valid_32_byte_hash() {
        let result = compute_sighash(1, &demo_action_hex()).expect("should succeed");
        assert_eq!(result.sighash_hex.len(), 64, "32 bytes = 64 hex chars");
        assert_eq!(result.seqno, 1);
    }

    #[test]
    fn test_compute_sighash_deterministic() {
        let h1 = compute_sighash(1, &demo_action_hex()).expect("should succeed");
        let h2 = compute_sighash(1, &demo_action_hex()).expect("should succeed");
        assert_eq!(h1.sighash_hex, h2.sighash_hex);
    }

    #[test]
    fn test_compute_sighash_different_seqno() {
        let h1 = compute_sighash(1, &demo_action_hex()).expect("should succeed");
        let h2 = compute_sighash(2, &demo_action_hex()).expect("should succeed");
        assert_ne!(h1.sighash_hex, h2.sighash_hex);
    }

    // -- sign_sighash tests --------------------------------------------------

    #[test]
    fn test_sign_sighash_success() {
        let keys = generate_demo_keys(1);
        let sighash = compute_sighash(1, &demo_action_hex()).expect("sighash ok");

        let result = sign_sighash(&keys[0].secret_key_hex, &sighash.sighash_hex)
            .expect("signing should succeed");
        assert!(!result.signature_hex.is_empty());
        assert!(!result.public_key_hex.is_empty());
    }

    #[test]
    fn test_sign_sighash_invalid_secret_key() {
        let result = sign_sighash("not_valid_hex", &"aa".repeat(32));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid secret key"));
    }

    #[test]
    fn test_sign_sighash_invalid_sighash_length() {
        let keys = generate_demo_keys(1);
        let result = sign_sighash(&keys[0].secret_key_hex, "tooshort");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid sighash"));
    }

    // -- verify_threshold tests ----------------------------------------------

    #[test]
    fn test_verify_threshold_full_flow_2_of_3() {
        let keys = generate_demo_keys(3);
        let sighash = compute_sighash(1, &demo_action_hex()).expect("sighash ok");

        let sig0 = sign_sighash(&keys[0].secret_key_hex, &sighash.sighash_hex).expect("sign ok");
        let sig2 = sign_sighash(&keys[2].secret_key_hex, &sighash.sighash_hex).expect("sign ok");

        let pub_keys: Vec<String> = keys.iter().map(|k| k.public_key_hex.clone()).collect();
        let result = verify_threshold(
            &pub_keys,
            2,
            &[sig0.signature_hex, sig2.signature_hex],
            &sighash.sighash_hex,
        )
        .expect("verify ok");

        assert!(result.valid);
        assert_eq!(result.signatures_verified, 2);
        assert_eq!(result.threshold_required, 2);
    }

    #[test]
    fn test_verify_threshold_below_threshold() {
        let keys = generate_demo_keys(3);
        let sighash = compute_sighash(1, &demo_action_hex()).expect("sighash ok");

        let sig0 = sign_sighash(&keys[0].secret_key_hex, &sighash.sighash_hex).expect("sign ok");

        let pub_keys: Vec<String> = keys.iter().map(|k| k.public_key_hex.clone()).collect();
        let result = verify_threshold(&pub_keys, 2, &[sig0.signature_hex], &sighash.sighash_hex)
            .expect("verify ok");

        assert!(!result.valid);
        assert_eq!(result.signatures_verified, 1);
    }

    #[test]
    fn test_verify_threshold_empty_signatures() {
        let keys = generate_demo_keys(3);
        let sighash = compute_sighash(1, &demo_action_hex()).expect("sighash ok");

        let pub_keys: Vec<String> = keys.iter().map(|k| k.public_key_hex.clone()).collect();
        let result = verify_threshold(&pub_keys, 2, &[], &sighash.sighash_hex).expect("verify ok");

        assert!(!result.valid);
        assert_eq!(result.signatures_verified, 0);
    }

    #[test]
    fn test_mnemonic_signature_verifies_against_raw_sighash() {
        // Regression: sign_with_mnemonic_path previously used signed_msg_hash (Bitcoin message
        // prefix), producing a signature that the ASM validator would silently reject.
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let path = "m/86'/0'/73'/0/0";
        let sighash = compute_sighash(1, &demo_action_hex()).expect("sighash ok");

        let result =
            sign_with_mnemonic_path(mnemonic, "", path, &sighash.sighash_hex).expect("sign ok");

        // 64-byte compact r||s — verify directly against raw sighash bytes.
        let sig_bytes = hex::decode(&result.signature_hex).expect("hex ok");
        assert_eq!(
            sig_bytes.len(),
            64,
            "mnemonic path must produce 64-byte compact sig"
        );

        let pk_bytes = hex::decode(&result.public_key_hex).expect("hex ok");
        let pk = bitcoin::secp256k1::PublicKey::from_slice(&pk_bytes).expect("pk ok");
        let hash_bytes = hex::decode(&sighash.sighash_hex).expect("hex ok");
        let msg = Message::from_digest_slice(&hash_bytes).expect("msg ok");
        let sig = bitcoin::secp256k1::ecdsa::Signature::from_compact(&sig_bytes).expect("sig ok");

        SECP256K1
            .verify_ecdsa(&msg, &sig, &pk)
            .expect("signature must verify against raw sighash");
    }

    #[test]
    fn test_verify_threshold_invalid_signature() {
        let keys = generate_demo_keys(3);
        let sighash = compute_sighash(1, &demo_action_hex()).expect("sighash ok");

        let fake_sig = "00".repeat(64);

        let pub_keys: Vec<String> = keys.iter().map(|k| k.public_key_hex.clone()).collect();
        let result =
            verify_threshold(&pub_keys, 2, &[fake_sig], &sighash.sighash_hex).expect("verify ok");

        assert!(!result.valid);
        assert_eq!(result.signatures_verified, 0);
    }
}
