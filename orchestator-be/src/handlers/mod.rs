use crate::state::AppState;
use axum::{
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};

pub mod auth;
pub mod proposals;

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        // Auth (unauthenticated bootstrap)
        .route("/auth", post(auth::authenticate))
        // Proposals (require ValidSession)
        .route("/proposals", get(proposals::list_proposals))
        .route("/proposals", post(proposals::create_proposal))
        .route("/proposals/:action_id", get(proposals::get_proposal))
        .route(
            "/proposals/:action_id/approve",
            post(proposals::approve_action),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::authority::Authority;
    use crate::domain::session::Session;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::Utc;
    use http_body_util::BodyExt;
    use rand::rngs::OsRng;
    use secp256k1::{Message, PublicKey, SecretKey, SECP256K1};
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use tower::util::ServiceExt;

    fn test_config() -> crate::config::Config {
        crate::config::Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 0,
            strata_rpc_url: None,
            strata_rpc_method: "strata_getAdminState".to_string(),
        }
    }

    /// Returns a router pre-seeded with a valid session.
    /// Callers receive (router, session_token, ephemeral_sk) to construct signed requests.
    fn authed_app() -> (Router, AppState, String, SecretKey) {
        let state = AppState::new(test_config());
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        let eph_sk = SecretKey::from_slice(&bytes).unwrap();
        let eph_pk = PublicKey::from_secret_key(SECP256K1, &eph_sk);
        let token = "test-session-token".to_string();
        let session = Session {
            token: token.clone(),
            authority: Authority::StrataAdmin,
            signer_pubkey: "test-signer".into(),
            ephemeral_pubkey: hex::encode(eph_pk.serialize()),
            expires_at: Utc::now().timestamp() + 3600,
        };
        state
            .sessions
            .write()
            .unwrap()
            .insert(token.clone(), session);
        let app = router(state.clone());
        (app, state, token, eph_sk)
    }

    fn sign_req(token: &str, timestamp_ms: i64, eph_sk: &SecretKey) -> String {
        let mut h = Sha256::new();
        h.update(b"alpen-request:v1");
        h.update(token.as_bytes());
        h.update(timestamp_ms.to_be_bytes());
        let hash = h.finalize();
        let msg = Message::from_digest_slice(&hash).unwrap();
        hex::encode(SECP256K1.sign_ecdsa(&msg, eph_sk).serialize_compact())
    }

    fn authed_request(
        method: &str,
        uri: &str,
        body: Option<Value>,
        token: &str,
        eph_sk: &SecretKey,
    ) -> Request<Body> {
        let ts = Utc::now().timestamp_millis();
        let sig = sign_req(token, ts, eph_sk);
        let builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .header("x-session-timestamp", ts.to_string())
            .header("x-request-sig", sig);

        match body {
            Some(b) => builder
                .body(Body::from(serde_json::to_vec(&b).unwrap()))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        }
    }

    async fn response_json(resp: axum::response::Response) -> Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn create_body() -> Value {
        json!({
            "authority": "strata_admin",
            "seq_no": 1,
            "action_hex": "deadbeef",
            "signer_pubkey": "pubkey_a",
            "signature_hex": "sig_a"
        })
    }

    // ─── auth: unauthenticated route reachable ────────────────────────────────

    #[tokio::test]
    async fn test_proposal_routes_require_auth() {
        let state = AppState::new(test_config());
        let app = router(state);
        // No auth headers → 401
        let req = Request::builder()
            .method("GET")
            .uri("/proposals")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // ─── create_proposal ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_proposal_happy_path() {
        let (app, _state, token, eph_sk) = authed_app();
        let req = authed_request("POST", "/proposals", Some(create_body()), &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = response_json(resp).await;
        assert_eq!(body["seq_no"], 1);
        assert_eq!(body["authority"], "strata_admin");
        assert_eq!(body["status"], "pending");
        assert_eq!(body["signatures"][0]["signer_pubkey"], "pubkey_a");
    }

    #[tokio::test]
    async fn test_create_proposal_duplicate_rejected() {
        let (_app, state, token, eph_sk) = authed_app();

        let app = router(state.clone());
        let req = authed_request("POST", "/proposals", Some(create_body()), &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let app = router(state);
        let req = authed_request("POST", "/proposals", Some(create_body()), &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_create_proposal_invalid_hex() {
        let (app, _state, token, eph_sk) = authed_app();
        let body = json!({
            "authority": "strata_admin",
            "seq_no": 1,
            "action_hex": "not_valid_hex",
            "signer_pubkey": "pubkey_a",
            "signature_hex": "sig_a"
        });
        let req = authed_request("POST", "/proposals", Some(body), &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ─── list_proposals ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_proposals_empty() {
        let (app, _state, token, eph_sk) = authed_app();
        let req = authed_request("GET", "/proposals", None, &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_json(resp).await;
        assert_eq!(body["proposals"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_list_proposals_returns_all() {
        let (_app, state, token, eph_sk) = authed_app();

        let app = router(state.clone());
        let req = authed_request("POST", "/proposals", Some(create_body()), &token, &eph_sk);
        app.oneshot(req).await.unwrap();

        let app = router(state.clone());
        let body2 = json!({
            "authority": "strata_admin",
            "seq_no": 2,
            "action_hex": "cafebabe",
            "signer_pubkey": "pubkey_a",
            "signature_hex": "sig_a"
        });
        let req = authed_request("POST", "/proposals", Some(body2), &token, &eph_sk);
        app.oneshot(req).await.unwrap();

        let app = router(state);
        let req = authed_request("GET", "/proposals", None, &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_json(resp).await;
        assert_eq!(body["proposals"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_list_proposals_filter_by_status() {
        let (_app, state, token, eph_sk) = authed_app();

        let app = router(state.clone());
        let req = authed_request("POST", "/proposals", Some(create_body()), &token, &eph_sk);
        app.oneshot(req).await.unwrap();

        let app = router(state.clone());
        let req = authed_request("GET", "/proposals?status=pending", None, &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();
        let body = response_json(resp).await;
        assert_eq!(body["proposals"].as_array().unwrap().len(), 1);

        let app = router(state);
        let req = authed_request("GET", "/proposals?status=approved", None, &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();
        let body = response_json(resp).await;
        assert_eq!(body["proposals"].as_array().unwrap().len(), 0);
    }

    // ─── get_proposal ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_proposal_happy_path() {
        let (_app, state, token, eph_sk) = authed_app();

        let app = router(state.clone());
        let req = authed_request("POST", "/proposals", Some(create_body()), &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();
        let created = response_json(resp).await;
        let action_id = created["action_id"].as_str().unwrap();

        let app = router(state);
        let req = authed_request(
            "GET",
            &format!("/proposals/{action_id}"),
            None,
            &token,
            &eph_sk,
        );
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_json(resp).await;
        assert_eq!(body["seq_no"], 1);
        assert_eq!(body["status"], "pending");
    }

    #[tokio::test]
    async fn test_get_proposal_not_found() {
        let (app, _state, token, eph_sk) = authed_app();
        let req = authed_request("GET", "/proposals/nonexistent", None, &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ─── approve_action ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_approve_action_happy_path() {
        let (_app, state, token, eph_sk) = authed_app();

        let app = router(state.clone());
        let req = authed_request("POST", "/proposals", Some(create_body()), &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();
        let created = response_json(resp).await;
        let action_id = created["action_id"].as_str().unwrap();

        let app = router(state);
        let sig_body = json!({ "signer_pubkey": "pubkey_b", "signature_hex": "sig_b" });
        let req = authed_request(
            "POST",
            &format!("/proposals/{action_id}/approve"),
            Some(sig_body),
            &token,
            &eph_sk,
        );
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_json(resp).await;
        assert_eq!(body["signatures"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_approve_action_duplicate_signer_rejected() {
        let (_app, state, token, eph_sk) = authed_app();

        let app = router(state.clone());
        let req = authed_request("POST", "/proposals", Some(create_body()), &token, &eph_sk);
        let resp = app.oneshot(req).await.unwrap();
        let created = response_json(resp).await;
        let action_id = created["action_id"].as_str().unwrap();

        let app = router(state);
        let sig_body = json!({ "signer_pubkey": "pubkey_a", "signature_hex": "sig_a_again" });
        let req = authed_request(
            "POST",
            &format!("/proposals/{action_id}/approve"),
            Some(sig_body),
            &token,
            &eph_sk,
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_approve_action_nonexistent_proposal() {
        let (app, _state, token, eph_sk) = authed_app();
        let sig_body = json!({ "signer_pubkey": "pubkey_a", "signature_hex": "sig_a" });
        let req = authed_request(
            "POST",
            "/proposals/nonexistent/approve",
            Some(sig_body),
            &token,
            &eph_sk,
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
