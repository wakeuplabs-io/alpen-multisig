use bitcoin::hashes::{sha256, Hash};
use bitcoin::secp256k1::{
    ecdsa::RecoverableSignature, ecdsa::RecoveryId, ecdsa::Signature, Message, PublicKey, Secp256k1,
};
use bitcoin::sign_message::signed_msg_hash;
use rand::RngCore;

const AUTH_DOMAIN: &str = "alpen-multisig/orchestrator/v1";

pub(crate) fn create_challenge_digest(
    authority_wire: &str,
    nonce_hex: &str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    challenge_id: &str,
) -> [u8; 32] {
    let payload = format!(
        "{AUTH_DOMAIN}|authority={authority_wire}|nonce={nonce_hex}|issued_at={issued_at_unix_ms}|expires_at={expires_at_unix_ms}|challenge_id={challenge_id}"
    );
    sha256::Hash::hash(payload.as_bytes()).to_byte_array()
}

pub(crate) fn random_hex(bytes_len: usize) -> String {
    let mut bytes = vec![0u8; bytes_len];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub(crate) fn verify_signature(
    challenge_digest_hex: &str,
    signer_pubkey_hex: &str,
    signature_hex: &str,
) -> Result<(), String> {
    let digest =
        hex::decode(challenge_digest_hex).map_err(|e| format!("invalid challenge hex: {e}"))?;
    let msg = Message::from_digest_slice(&digest)
        .map_err(|e| format!("challenge digest must be 32 bytes: {e}"))?;

    let signature_bytes =
        hex::decode(signature_hex).map_err(|e| format!("invalid signature hex: {e}"))?;
    let signature = Signature::from_compact(&signature_bytes)
        .map_err(|e| format!("invalid compact signature: {e}"))?;

    let pubkey_bytes =
        hex::decode(signer_pubkey_hex).map_err(|e| format!("invalid public key hex: {e}"))?;
    let pubkey =
        PublicKey::from_slice(&pubkey_bytes).map_err(|e| format!("invalid public key: {e}"))?;

    Secp256k1::new()
        .verify_ecdsa(&msg, &signature, &pubkey)
        .map_err(|e| format!("signature verification failed: {e}"))
}

pub(crate) fn verify_bitcoin_message_signature(
    challenge_digest_hex: &str,
    signer_pubkey_hex: &str,
    signature_hex: &str,
) -> Result<(), String> {
    let signature_bytes =
        hex::decode(signature_hex).map_err(|e| format!("invalid signature hex: {e}"))?;
    if signature_bytes.len() != 65 {
        return Err(format!(
            "invalid bitcoin-message signature length: expected 65 bytes, got {}",
            signature_bytes.len()
        ));
    }

    let recid_byte = *signature_bytes.last().ok_or("missing recovery id byte")?;
    let recid = RecoveryId::from_i32(recid_byte as i32)
        .map_err(|e| format!("invalid recovery id byte: {e}"))?;
    let recoverable = RecoverableSignature::from_compact(&signature_bytes[..64], recid)
        .map_err(|e| format!("invalid recoverable signature: {e}"))?;

    let message_hash = signed_msg_hash(challenge_digest_hex);
    let message = Message::from_digest_slice(message_hash.as_ref())
        .map_err(|e| format!("invalid signed message digest: {e}"))?;
    let recovered = Secp256k1::new()
        .recover_ecdsa(&message, &recoverable)
        .map_err(|e| format!("failed to recover signer pubkey: {e}"))?;

    let expected_pubkey = {
        let pubkey_bytes =
            hex::decode(signer_pubkey_hex).map_err(|e| format!("invalid public key hex: {e}"))?;
        PublicKey::from_slice(&pubkey_bytes).map_err(|e| format!("invalid public key: {e}"))?
    };
    if recovered != expected_pubkey {
        return Err("bitcoin-message signature pubkey mismatch".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{Message, PublicKey, Secp256k1, SecretKey};

    #[test]
    fn challenge_digest_is_deterministic() {
        let a = create_challenge_digest("strata_admin", "aa", 1, 2, "cid");
        let b = create_challenge_digest("strata_admin", "aa", 1, 2, "cid");
        assert_eq!(a, b);
    }

    #[test]
    fn verify_signature_accepts_signer_two_fixture() {
        let mut sk_bytes = [0u8; 32];
        sk_bytes[31] = 2;
        let sk = SecretKey::from_slice(&sk_bytes).expect("valid key");
        let pk = PublicKey::from_secret_key(&Secp256k1::new(), &sk);

        let digest_hex = hex::encode(create_challenge_digest("strata_admin", "11", 1, 1000, "id"));
        let digest = hex::decode(&digest_hex).expect("digest hex");
        let msg = Message::from_digest_slice(&digest).expect("digest message");
        let signature = Secp256k1::new().sign_ecdsa(&msg, &sk);
        let signature_hex = hex::encode(signature.serialize_compact());

        verify_signature(&digest_hex, &hex::encode(pk.serialize()), &signature_hex)
            .expect("signature should verify");
    }
}
