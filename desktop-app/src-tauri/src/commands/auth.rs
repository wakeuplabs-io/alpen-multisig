use bitcoin::secp256k1::{Message, PublicKey, SecretKey, SECP256K1};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

use crate::state::AppState;

const MIN_SESSION_SECS: u64 = 60;
const MAX_SESSION_SECS: u64 = 8 * 3600;
const DEFAULT_SESSION_SECS: u64 = 3600;

#[derive(Debug, Serialize)]
pub struct AuthResult {
    pub session_token: String,
    pub expires_at: i64,
}

#[derive(Debug, Deserialize)]
struct AuthResponseBody {
    session_token: String,
    expires_at: i64,
}

/// Authenticate the signer with the backend and establish an ephemeral session.
///
/// Steps:
/// 1. Generate ephemeral keypair (software key — future: route through HW wallet)
/// 2. Sign auth message binding ephemeral key + authority + nonce + expiry
/// 3. Call backend /auth endpoint, obtain session token
/// 4. Store session token + ephemeral key in Tauri state
///
/// NOTE: `signer_secret_key_hex` is passed over the Tauri IPC bridge (via JSON).
/// Hardware wallet signing for the auth message is a future story.
#[tauri::command]
pub(crate) async fn authenticate(
    state: State<'_, AppState>,
    signer_secret_key_hex: String,
    expiry_secs: Option<u64>,
) -> Result<AuthResult, String> {
    // 1. Resolve authority from state.
    let authority = state
        .selected_authority
        .lock()
        .map_err(|_| "selected_authority lock poisoned".to_string())?
        .clone()
        .ok_or("No authority selected — call set_selected_authority first")?;

    // 2. Generate ephemeral keypair.
    let eph_sk = SecretKey::new(&mut OsRng);
    let eph_pk = PublicKey::from_secret_key(SECP256K1, &eph_sk);
    let eph_pk_hex = hex::encode(eph_pk.serialize());

    // 3. Build expiry (clamp to [MIN, MAX]).
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("system clock error: {e}"))?
        .as_secs();
    let requested = expiry_secs.unwrap_or(DEFAULT_SESSION_SECS);
    let expiry_secs_clamped = now_secs + requested.clamp(MIN_SESSION_SECS, MAX_SESSION_SECS);

    // 4. Generate 32-byte nonce.
    let mut nonce_bytes = [0u8; 32];
    use rand::RngCore;
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce_hex = hex::encode(nonce_bytes);

    // 5. Hash auth message.
    let eph_pk_bytes = eph_pk.serialize();
    let mut hasher = Sha256::new();
    hasher.update(b"alpen-multisig:auth-v1");
    hasher.update(authority.as_bytes());
    hasher.update(eph_pk_bytes);
    hasher.update(nonce_bytes);
    hasher.update(expiry_secs_clamped.to_be_bytes());
    let hash = hasher.finalize();

    // 6. Sign auth message with signer key.
    let signer_sk_bytes =
        hex::decode(&signer_secret_key_hex).map_err(|e| format!("invalid signer key hex: {e}"))?;
    let signer_sk = SecretKey::from_slice(&signer_sk_bytes)
        .map_err(|e| format!("invalid signer secret key: {e}"))?;
    let signer_pk = PublicKey::from_secret_key(SECP256K1, &signer_sk);
    let signer_pk_hex = hex::encode(signer_pk.serialize());

    let msg = Message::from_digest_slice(&hash).map_err(|e| format!("hash error: {e}"))?;
    let sig = SECP256K1.sign_ecdsa(&msg, &signer_sk);
    let sig_hex = hex::encode(sig.serialize_compact());

    // 7. POST to backend /auth.
    let backend_url = state.backend_url.clone();
    let request_body = serde_json::json!({
        "authority": authority,
        "signer_pubkey": signer_pk_hex,
        "ephemeral_pubkey": eph_pk_hex,
        "nonce": nonce_hex,
        "expiry_secs": expiry_secs_clamped,
        "signature": sig_hex,
    });

    let resp = reqwest::Client::new()
        .post(format!("{backend_url}/auth"))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("auth request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("auth failed ({status}): {body}"));
    }

    let auth_resp: AuthResponseBody = resp
        .json()
        .await
        .map_err(|e| format!("failed to parse auth response: {e}"))?;

    // 8. Store session token and ephemeral key in Tauri state.
    *state
        .session_token
        .lock()
        .map_err(|_| "session_token lock poisoned")? = Some(auth_resp.session_token.clone());

    *state
        .ephemeral_secret_key
        .lock()
        .map_err(|_| "ephemeral_secret_key lock poisoned")? = Some(eph_sk);

    *state
        .ephemeral_pubkey_hex
        .lock()
        .map_err(|_| "ephemeral_pubkey_hex lock poisoned")? = Some(eph_pk_hex);

    Ok(AuthResult {
        session_token: auth_resp.session_token,
        expires_at: auth_resp.expires_at,
    })
}
