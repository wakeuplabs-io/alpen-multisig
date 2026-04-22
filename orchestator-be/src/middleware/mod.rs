use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use secp256k1::{ecdsa::Signature, Message, PublicKey, SECP256K1};
use sha2::{Digest, Sha256};

use crate::domain::authority::Authority;
use crate::error::AppError;
use crate::state::AppState;

/// Request timestamp freshness window: ±60 seconds.
const TIMESTAMP_WINDOW_MS: i64 = 60_000;

/// Verified session extracted from Bearer token + ephemeral key signature.
/// Handlers declare this as a parameter to require authentication.
/// Fields are `pub` for handlers that need authority-scoped filtering (future stories).
#[allow(dead_code)]
pub struct ValidSession {
    pub token: String,
    pub authority: Authority,
    pub signer_pubkey: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for ValidSession
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let token = extract_bearer(parts).map_err(|e| e.into_response())?;

        let timestamp_ms = extract_header_str(parts, "x-session-timestamp")
            .and_then(|v| {
                v.parse::<i64>()
                    .map_err(|_| "x-session-timestamp must be an integer".to_string())
            })
            .map_err(|msg| AppError::Unauthorized(msg).into_response())?;

        let req_sig_hex = extract_header_str(parts, "x-request-sig")
            .map_err(|msg| AppError::Unauthorized(msg).into_response())?;

        let session = {
            let sessions = app_state.sessions.read().map_err(|_| {
                AppError::Internal(anyhow::anyhow!("sessions lock poisoned")).into_response()
            })?;
            sessions.get(&token).cloned()
        };

        let session = session
            .ok_or_else(|| AppError::Unauthorized("session not found".into()).into_response())?;

        let now = Utc::now().timestamp();
        if session.expires_at < now {
            return Err(AppError::Unauthorized("session expired".into()).into_response());
        }

        let now_ms = Utc::now().timestamp_millis();
        if (now_ms - timestamp_ms).abs() > TIMESTAMP_WINDOW_MS {
            return Err(AppError::Unauthorized("stale request timestamp".into()).into_response());
        }

        let eph_pk_bytes = hex::decode(&session.ephemeral_pubkey).map_err(|_| {
            AppError::Unauthorized("invalid stored ephemeral pubkey".into()).into_response()
        })?;
        let eph_pk = PublicKey::from_slice(&eph_pk_bytes).map_err(|_| {
            AppError::Unauthorized("invalid stored ephemeral pubkey".into()).into_response()
        })?;

        let sig_bytes = hex::decode(&req_sig_hex).map_err(|_| {
            AppError::Unauthorized("invalid x-request-sig hex".into()).into_response()
        })?;
        let sig = Signature::from_compact(&sig_bytes).map_err(|_| {
            AppError::Unauthorized("invalid x-request-sig ECDSA signature".into()).into_response()
        })?;

        let mut hasher = Sha256::new();
        hasher.update(b"alpen-request:v1");
        hasher.update(token.as_bytes());
        hasher.update(timestamp_ms.to_be_bytes());
        let hash = hasher.finalize();

        let msg = Message::from_digest_slice(&hash).map_err(|_| {
            AppError::Internal(anyhow::anyhow!("request hash invalid")).into_response()
        })?;

        SECP256K1.verify_ecdsa(&msg, &sig, &eph_pk).map_err(|_| {
            AppError::Unauthorized("invalid request signature".into()).into_response()
        })?;

        Ok(ValidSession {
            token,
            authority: session.authority,
            signer_pubkey: session.signer_pubkey,
        })
    }
}

fn extract_bearer(parts: &Parts) -> Result<String, AppError> {
    let value = parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing Authorization header".into()))?;

    value
        .strip_prefix("Bearer ")
        .map(|t| t.to_string())
        .ok_or_else(|| AppError::Unauthorized("Authorization must be Bearer <token>".into()))
}

fn extract_header_str(parts: &Parts, name: &str) -> Result<String, String> {
    parts
        .headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing {name} header"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::session::Session;
    use crate::state::AppState;
    use axum::{body::Body, http::Request, routing::get, Router};
    use chrono::Utc;
    use rand::rngs::OsRng;
    use secp256k1::{SecretKey, SECP256K1};
    use tower::util::ServiceExt;

    fn test_config() -> crate::config::Config {
        crate::config::Config {
            server_host: "127.0.0.1".into(),
            server_port: 0,
            strata_rpc_url: None,
            strata_rpc_method: "strata_getAdminState".into(),
        }
    }

    fn make_session(token: &str, eph_pubkey_hex: &str, expires_at: i64) -> Session {
        Session {
            token: token.to_string(),
            authority: Authority::StrataAdmin,
            signer_pubkey: "any".into(),
            ephemeral_pubkey: eph_pubkey_hex.to_string(),
            expires_at,
        }
    }

    pub fn generate_eph_keypair() -> (SecretKey, String) {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let sk = SecretKey::from_slice(&bytes).unwrap();
        let pk = secp256k1::PublicKey::from_secret_key(SECP256K1, &sk);
        (sk, hex::encode(pk.serialize()))
    }

    pub fn sign_request_headers(token: &str, timestamp_ms: i64, eph_sk: &SecretKey) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"alpen-request:v1");
        hasher.update(token.as_bytes());
        hasher.update(timestamp_ms.to_be_bytes());
        let hash = hasher.finalize();
        let msg = Message::from_digest_slice(&hash).unwrap();
        let sig = SECP256K1.sign_ecdsa(&msg, eph_sk);
        hex::encode(sig.serialize_compact())
    }

    fn test_router(state: AppState) -> Router {
        async fn probe(_session: ValidSession) -> &'static str {
            "ok"
        }
        Router::new().route("/probe", get(probe)).with_state(state)
    }

    fn authed_get(token: &str, timestamp_ms: i64, req_sig: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri("/probe")
            .header("authorization", format!("Bearer {token}"))
            .header("x-session-timestamp", timestamp_ms.to_string())
            .header("x-request-sig", req_sig)
            .body(Body::empty())
            .unwrap()
    }

    async fn status_code(app: Router, req: Request<Body>) -> u16 {
        app.oneshot(req).await.unwrap().status().as_u16()
    }

    #[tokio::test]
    async fn test_valid_session_accepted() {
        let state = AppState::new(test_config());
        let (eph_sk, eph_pk) = generate_eph_keypair();
        let token = "valid-token";
        let ts = Utc::now().timestamp_millis();
        let sig = sign_request_headers(token, ts, &eph_sk);

        state.sessions.write().unwrap().insert(
            token.into(),
            make_session(token, &eph_pk, Utc::now().timestamp() + 3600),
        );

        assert_eq!(
            status_code(test_router(state), authed_get(token, ts, &sig)).await,
            200
        );
    }

    #[tokio::test]
    async fn test_missing_auth_header_returns_401() {
        let state = AppState::new(test_config());
        let req = Request::builder()
            .method("GET")
            .uri("/probe")
            .body(Body::empty())
            .unwrap();
        assert_eq!(status_code(test_router(state), req).await, 401);
    }

    #[tokio::test]
    async fn test_expired_session_returns_401() {
        let state = AppState::new(test_config());
        let (eph_sk, eph_pk) = generate_eph_keypair();
        let token = "expired-token";
        let ts = Utc::now().timestamp_millis();
        let sig = sign_request_headers(token, ts, &eph_sk);

        state.sessions.write().unwrap().insert(
            token.into(),
            make_session(token, &eph_pk, Utc::now().timestamp() - 10),
        );

        assert_eq!(
            status_code(test_router(state), authed_get(token, ts, &sig)).await,
            401
        );
    }

    #[tokio::test]
    async fn test_stale_timestamp_returns_401() {
        let state = AppState::new(test_config());
        let (eph_sk, eph_pk) = generate_eph_keypair();
        let token = "stale-token";
        let stale_ts = Utc::now().timestamp_millis() - 120_000;
        let sig = sign_request_headers(token, stale_ts, &eph_sk);

        state.sessions.write().unwrap().insert(
            token.into(),
            make_session(token, &eph_pk, Utc::now().timestamp() + 3600),
        );

        assert_eq!(
            status_code(test_router(state), authed_get(token, stale_ts, &sig)).await,
            401
        );
    }

    #[tokio::test]
    async fn test_bad_request_sig_returns_401() {
        let state = AppState::new(test_config());
        let (_eph_sk, eph_pk) = generate_eph_keypair();
        let token = "badsig-token";
        let ts = Utc::now().timestamp_millis();

        state.sessions.write().unwrap().insert(
            token.into(),
            make_session(token, &eph_pk, Utc::now().timestamp() + 3600),
        );

        assert_eq!(
            status_code(test_router(state), authed_get(token, ts, &"00".repeat(64))).await,
            401
        );
    }

    #[tokio::test]
    async fn test_unknown_token_returns_401() {
        let state = AppState::new(test_config());
        let (eph_sk, _) = generate_eph_keypair();
        let token = "unknown-token";
        let ts = Utc::now().timestamp_millis();
        let sig = sign_request_headers(token, ts, &eph_sk);

        assert_eq!(
            status_code(test_router(state), authed_get(token, ts, &sig)).await,
            401
        );
    }
}
