use bitcoin::hashes::{sha256, Hash};
use bitcoin::secp256k1::{
    ecdsa::RecoverableSignature, ecdsa::RecoveryId, ecdsa::Signature, Message, PublicKey, SECP256K1,
};
use bitcoin::sign_message::signed_msg_hash;
use rand::RngCore;

const AUTH_DOMAIN: &str = "alpen-multisig/auth/v1";

pub fn render_challenge_message(role_label: &str, challenge_hex: &str) -> String {
    format!("Strata Session Authentication v1\nRole: {role_label}\nChallenge: {challenge_hex}")
}

pub fn create_challenge_digest(
    role_wire: &str,
    nonce_hex: &str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    session_id: &str,
) -> [u8; 32] {
    let payload = format!(
        "{AUTH_DOMAIN}|role={role_wire}|nonce={nonce_hex}|issued_at={issued_at_unix_ms}|expires_at={expires_at_unix_ms}|session_id={session_id}"
    );
    sha256::Hash::hash(payload.as_bytes()).to_byte_array()
}

pub fn random_nonce_hex() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn random_session_id() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn verify_signature(
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

    SECP256K1
        .verify_ecdsa(&msg, &signature, &pubkey)
        .map_err(|e| format!("signature verification failed: {e}"))
}

pub fn verify_bitcoin_message_signature(
    challenge_message: &str,
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

    let message_hash = signed_msg_hash(challenge_message);
    let msg = Message::from_digest_slice(&message_hash.to_byte_array())
        .map_err(|e| format!("invalid bitcoin message digest: {e}"))?;
    let recovered = SECP256K1
        .recover_ecdsa(&msg, &recoverable)
        .map_err(|e| format!("failed to recover signer pubkey: {e}"))?;

    let recovered_hex = hex::encode(recovered.serialize());
    if !recovered_hex.eq_ignore_ascii_case(signer_pubkey_hex) {
        return Err("bitcoin-message signature does not match provided signer pubkey".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{Secp256k1, SecretKey};

    #[test]
    fn create_challenge_digest_is_stable() {
        let d1 = create_challenge_digest("strata_administrator", "ab", 10, 20, "sess");
        let d2 = create_challenge_digest("strata_administrator", "ab", 10, 20, "sess");
        assert_eq!(d1, d2);
    }

    #[test]
    fn verify_signature_accepts_valid_signature() {
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[42u8; 32]).expect("valid key");
        let pk = PublicKey::from_secret_key(&secp, &sk);

        let digest = create_challenge_digest("strata_administrator", "aa", 1, 2, "sid");
        let msg = Message::from_digest_slice(&digest).expect("32-byte digest");
        let signature = secp.sign_ecdsa(&msg, &sk);

        verify_signature(
            &hex::encode(digest),
            &hex::encode(pk.serialize()),
            &hex::encode(signature.serialize_compact()),
        )
        .expect("valid signature must verify");
    }

    #[test]
    fn verify_signature_rejects_invalid_signature() {
        let err = verify_signature("00", "11", "22").unwrap_err();
        assert!(err.contains("challenge digest must be 32 bytes"));
    }
}
