use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::{Arc, RwLock};

use chrono::Utc;
use secp256k1::{ecdsa::Signature, Message, PublicKey, SECP256K1};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::Config;
use crate::domain::authority::Authority;
use crate::domain::session::Session;
use crate::error::{AppError, Result};

const MAX_SESSION_SECS: i64 = 8 * 3600;

pub struct AuthRequest {
    pub authority: Authority,
    pub signer_pubkey: String,
    pub ephemeral_pubkey: String,
    pub nonce: String,
    pub expiry_secs: u64,
    pub signature: String,
}

#[derive(Debug)]
pub struct AuthResponse {
    pub session_token: String,
    pub expires_at: i64,
}

pub async fn authenticate<F, Fut>(
    req: AuthRequest,
    _config: &Config,
    sessions: &Arc<RwLock<HashMap<String, Session>>>,
    used_nonces: &Arc<RwLock<HashSet<[u8; 32]>>>,
    fetch_signer_set: F,
) -> Result<AuthResponse>
where
    F: FnOnce(Authority) -> Fut,
    Fut: Future<Output = anyhow::Result<Vec<String>>>,
{
    let now = Utc::now().timestamp();

    // --- decode inputs ---

    let signer_pk_bytes = hex::decode(&req.signer_pubkey)
        .map_err(|_| AppError::BadRequest("invalid signer_pubkey hex".into()))?;
    let signer_pk = PublicKey::from_slice(&signer_pk_bytes)
        .map_err(|_| AppError::BadRequest("invalid signer_pubkey secp256k1 key".into()))?;

    let eph_pk_bytes = hex::decode(&req.ephemeral_pubkey)
        .map_err(|_| AppError::BadRequest("invalid ephemeral_pubkey hex".into()))?;
    let _eph_pk = PublicKey::from_slice(&eph_pk_bytes)
        .map_err(|_| AppError::BadRequest("invalid ephemeral_pubkey secp256k1 key".into()))?;

    let nonce_bytes =
        hex::decode(&req.nonce).map_err(|_| AppError::BadRequest("invalid nonce hex".into()))?;
    if nonce_bytes.len() != 32 {
        return Err(AppError::BadRequest("nonce must be 32 bytes".into()));
    }
    let nonce_arr: [u8; 32] = nonce_bytes.try_into().unwrap();

    let sig_bytes = hex::decode(&req.signature)
        .map_err(|_| AppError::BadRequest("invalid signature hex".into()))?;
    let sig = Signature::from_compact(&sig_bytes)
        .map_err(|_| AppError::BadRequest("invalid ECDSA signature".into()))?;

    // --- expiry validation ---

    let expiry = req.expiry_secs as i64;
    if expiry <= now {
        return Err(AppError::BadRequest("expiry is in the past".into()));
    }
    if expiry > now + MAX_SESSION_SECS {
        return Err(AppError::BadRequest(format!(
            "expiry exceeds maximum allowed session duration of {} hours",
            MAX_SESSION_SECS / 3600
        )));
    }

    // --- nonce replay check ---

    {
        let mut nonces = used_nonces
            .write()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("nonce lock poisoned")))?;
        if nonces.contains(&nonce_arr) {
            return Err(AppError::BadRequest("nonce already used".into()));
        }
        nonces.insert(nonce_arr);
    }

    // --- reconstruct and verify auth message ---

    let authority_str = req.authority.as_str();
    let expiry_be8 = req.expiry_secs.to_be_bytes();

    let mut hasher = Sha256::new();
    hasher.update(b"alpen-multisig:auth-v1");
    hasher.update(authority_str.as_bytes());
    hasher.update(&eph_pk_bytes);
    hasher.update(nonce_arr);
    hasher.update(expiry_be8);
    let hash = hasher.finalize();

    let msg = Message::from_digest_slice(&hash)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("auth message hash is invalid")))?;

    SECP256K1
        .verify_ecdsa(&msg, &sig, &signer_pk)
        .map_err(|_| AppError::Unauthorized("invalid signature".into()))?;

    // --- canonical signer set check ---

    let signer_set = fetch_signer_set(req.authority)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to fetch signer set: {e}")))?;

    let signer_pubkey_normalised = hex::encode(signer_pk.serialize());
    if !signer_set.contains(&signer_pubkey_normalised) {
        return Err(AppError::Forbidden("signer not in authority set".into()));
    }

    // --- issue session ---

    let token = Uuid::new_v4().to_string();
    let session = Session {
        token: token.clone(),
        authority: req.authority,
        signer_pubkey: signer_pubkey_normalised,
        ephemeral_pubkey: hex::encode(&eph_pk_bytes),
        expires_at: expiry,
    };

    sessions
        .write()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("sessions lock poisoned")))?
        .insert(token.clone(), session);

    Ok(AuthResponse {
        session_token: token,
        expires_at: expiry,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use rand::rngs::OsRng;
    use secp256k1::{SecretKey, SECP256K1};

    fn test_config() -> Config {
        Config {
            server_host: "127.0.0.1".into(),
            server_port: 3000,
            strata_rpc_url: Some("http://localhost:9999".into()),
            strata_rpc_method: "strata_getAdminState".into(),
        }
    }

    fn generate_keypair() -> (SecretKey, String) {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let sk = SecretKey::from_slice(&bytes).unwrap();
        let pk = secp256k1::PublicKey::from_secret_key(SECP256K1, &sk);
        (sk, hex::encode(pk.serialize()))
    }

    fn make_nonce() -> [u8; 32] {
        let mut nonce = [0u8; 32];
        use rand::RngCore;
        OsRng.fill_bytes(&mut nonce);
        nonce
    }

    fn sign_auth_msg(
        sk: &SecretKey,
        authority: Authority,
        eph_pubkey_hex: &str,
        nonce: &[u8; 32],
        expiry_secs: u64,
    ) -> String {
        let eph_bytes = hex::decode(eph_pubkey_hex).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(b"alpen-multisig:auth-v1");
        hasher.update(authority.as_str().as_bytes());
        hasher.update(&eph_bytes);
        hasher.update(nonce);
        hasher.update(expiry_secs.to_be_bytes());
        let hash = hasher.finalize();
        let msg = Message::from_digest_slice(&hash).unwrap();
        let sig = SECP256K1.sign_ecdsa(&msg, sk);
        hex::encode(sig.serialize_compact())
    }

    fn make_state() -> (
        Config,
        Arc<RwLock<HashMap<String, Session>>>,
        Arc<RwLock<HashSet<[u8; 32]>>>,
    ) {
        (
            test_config(),
            Arc::new(RwLock::new(HashMap::new())),
            Arc::new(RwLock::new(HashSet::new())),
        )
    }

    #[tokio::test]
    async fn test_happy_path() {
        let (config, sessions, nonces) = make_state();
        let (signer_sk, signer_pk) = generate_keypair();
        let (_eph_sk, eph_pk) = generate_keypair();
        let nonce = make_nonce();
        let expiry = (Utc::now().timestamp() + 3600) as u64;
        let sig = sign_auth_msg(&signer_sk, Authority::StrataAdmin, &eph_pk, &nonce, expiry);

        let signer_pk_clone = signer_pk.clone();
        let result = authenticate(
            AuthRequest {
                authority: Authority::StrataAdmin,
                signer_pubkey: signer_pk.clone(),
                ephemeral_pubkey: eph_pk,
                nonce: hex::encode(nonce),
                expiry_secs: expiry,
                signature: sig,
            },
            &config,
            &sessions,
            &nonces,
            |_auth| async move { Ok(vec![signer_pk_clone]) },
        )
        .await
        .expect("should succeed");

        assert!(!result.session_token.is_empty());
        assert_eq!(result.expires_at, expiry as i64);

        let locked = sessions.read().unwrap();
        assert!(locked.contains_key(&result.session_token));
    }

    #[tokio::test]
    async fn test_invalid_signature_returns_unauthorized() {
        let (config, sessions, nonces) = make_state();
        let (_signer_sk, signer_pk) = generate_keypair();
        let (_eph_sk, eph_pk) = generate_keypair();
        let nonce = make_nonce();
        let expiry = (Utc::now().timestamp() + 3600) as u64;
        let bad_sig = "00".repeat(64);

        let signer_pk_clone = signer_pk.clone();
        let err = authenticate(
            AuthRequest {
                authority: Authority::StrataAdmin,
                signer_pubkey: signer_pk,
                ephemeral_pubkey: eph_pk,
                nonce: hex::encode(nonce),
                expiry_secs: expiry,
                signature: bad_sig,
            },
            &config,
            &sessions,
            &nonces,
            |_| async move { Ok(vec![signer_pk_clone]) },
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn test_signer_not_in_set_returns_forbidden() {
        let (config, sessions, nonces) = make_state();
        let (signer_sk, signer_pk) = generate_keypair();
        let (_eph_sk, eph_pk) = generate_keypair();
        let nonce = make_nonce();
        let expiry = (Utc::now().timestamp() + 3600) as u64;
        let sig = sign_auth_msg(&signer_sk, Authority::StrataAdmin, &eph_pk, &nonce, expiry);

        let err = authenticate(
            AuthRequest {
                authority: Authority::StrataAdmin,
                signer_pubkey: signer_pk,
                ephemeral_pubkey: eph_pk,
                nonce: hex::encode(nonce),
                expiry_secs: expiry,
                signature: sig,
            },
            &config,
            &sessions,
            &nonces,
            |_| async move { Ok(vec![]) }, // empty signer set
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn test_past_expiry_returns_bad_request() {
        let (config, sessions, nonces) = make_state();
        let (signer_sk, signer_pk) = generate_keypair();
        let (_eph_sk, eph_pk) = generate_keypair();
        let nonce = make_nonce();
        let expiry = (Utc::now().timestamp() - 1) as u64; // past
        let sig = sign_auth_msg(&signer_sk, Authority::StrataAdmin, &eph_pk, &nonce, expiry);

        let err = authenticate(
            AuthRequest {
                authority: Authority::StrataAdmin,
                signer_pubkey: signer_pk.clone(),
                ephemeral_pubkey: eph_pk,
                nonce: hex::encode(nonce),
                expiry_secs: expiry,
                signature: sig,
            },
            &config,
            &sessions,
            &nonces,
            |_| async move { Ok(vec![signer_pk]) },
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_expiry_too_far_returns_bad_request() {
        let (config, sessions, nonces) = make_state();
        let (signer_sk, signer_pk) = generate_keypair();
        let (_eph_sk, eph_pk) = generate_keypair();
        let nonce = make_nonce();
        let expiry = (Utc::now().timestamp() + MAX_SESSION_SECS + 100) as u64;
        let sig = sign_auth_msg(&signer_sk, Authority::StrataAdmin, &eph_pk, &nonce, expiry);

        let err = authenticate(
            AuthRequest {
                authority: Authority::StrataAdmin,
                signer_pubkey: signer_pk.clone(),
                ephemeral_pubkey: eph_pk,
                nonce: hex::encode(nonce),
                expiry_secs: expiry,
                signature: sig,
            },
            &config,
            &sessions,
            &nonces,
            |_| async move { Ok(vec![signer_pk]) },
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_replayed_nonce_returns_bad_request() {
        let (config, sessions, nonces) = make_state();
        let (signer_sk, signer_pk) = generate_keypair();
        let (_eph_sk, eph_pk) = generate_keypair();
        let nonce = make_nonce();
        let expiry = (Utc::now().timestamp() + 3600) as u64;
        let sig = sign_auth_msg(&signer_sk, Authority::StrataAdmin, &eph_pk, &nonce, expiry);

        let signer_pk_clone = signer_pk.clone();
        let eph_pk_clone = eph_pk.clone();
        authenticate(
            AuthRequest {
                authority: Authority::StrataAdmin,
                signer_pubkey: signer_pk.clone(),
                ephemeral_pubkey: eph_pk.clone(),
                nonce: hex::encode(nonce),
                expiry_secs: expiry,
                signature: sig.clone(),
            },
            &config,
            &sessions,
            &nonces,
            |_| async move { Ok(vec![signer_pk_clone]) },
        )
        .await
        .expect("first call should succeed");

        let signer_pk_clone2 = signer_pk.clone();
        let err = authenticate(
            AuthRequest {
                authority: Authority::StrataAdmin,
                signer_pubkey: signer_pk,
                ephemeral_pubkey: eph_pk_clone,
                nonce: hex::encode(nonce),
                expiry_secs: expiry,
                signature: sig,
            },
            &config,
            &sessions,
            &nonces,
            |_| async move { Ok(vec![signer_pk_clone2]) },
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_bad_nonce_length_returns_bad_request() {
        let (config, sessions, nonces) = make_state();
        let (_sk, signer_pk) = generate_keypair();
        let (_eph_sk, eph_pk) = generate_keypair();
        let expiry = (Utc::now().timestamp() + 3600) as u64;

        let err = authenticate(
            AuthRequest {
                authority: Authority::StrataAdmin,
                signer_pubkey: signer_pk.clone(),
                ephemeral_pubkey: eph_pk,
                nonce: hex::encode([0u8; 16]), // wrong length
                expiry_secs: expiry,
                signature: "00".repeat(64),
            },
            &config,
            &sessions,
            &nonces,
            |_| async move { Ok(vec![signer_pk]) },
        )
        .await
        .unwrap_err();

        assert!(matches!(err, AppError::BadRequest(_)));
    }
}
