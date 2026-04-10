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
        // Auth
        .route("/auth/challenge", get(auth::get_challenge))
        .route("/auth/session", post(auth::create_session))
        .route("/auth/session", axum::routing::delete(auth::delete_session))
        // Proposals
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
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::util::ServiceExt;

    fn test_app() -> Router {
        let config = crate::config::Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 0,
        };
        let state = AppState::new(config);
        router(state)
    }

    fn json_request(method: &str, uri: &str, body: Option<Value>) -> Request<Body> {
        let builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");

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

    // ─── create_proposal ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_proposal_happy_path() {
        let app = test_app();
        let req = json_request("POST", "/proposals", Some(create_body()));
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);

        let body = response_json(resp).await;
        assert!(body["action_id"].is_string());
        assert_eq!(body["proposal"]["seq_no"], 1);
        assert_eq!(body["proposal"]["authority"], "strata_admin");
        assert_eq!(body["proposal"]["status"], "pending");
        assert_eq!(
            body["proposal"]["signatures"][0]["signer_pubkey"],
            "pubkey_a"
        );
    }

    #[tokio::test]
    async fn test_create_proposal_duplicate_rejected() {
        let config = crate::config::Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 0,
        };
        let state = AppState::new(config);
        let app = router(state.clone());

        // First create
        let req = json_request("POST", "/proposals", Some(create_body()));
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // Second create — same action_hex + seq_no
        let app = router(state);
        let req = json_request("POST", "/proposals", Some(create_body()));
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_create_proposal_invalid_hex() {
        let app = test_app();
        let body = json!({
            "authority": "strata_admin",
            "seq_no": 1,
            "action_hex": "not_valid_hex",
            "signer_pubkey": "pubkey_a",
            "signature_hex": "sig_a"
        });
        let req = json_request("POST", "/proposals", Some(body));
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ─── list_proposals ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_proposals_empty() {
        let app = test_app();
        let req = json_request("GET", "/proposals", None);
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_json(resp).await;
        assert_eq!(body["proposals"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_list_proposals_returns_all() {
        let config = crate::config::Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 0,
        };
        let state = AppState::new(config);

        // Create 2 proposals
        let app = router(state.clone());
        let req = json_request("POST", "/proposals", Some(create_body()));
        app.oneshot(req).await.unwrap();

        let app = router(state.clone());
        let body2 = json!({
            "authority": "strata_admin",
            "seq_no": 2,
            "action_hex": "cafebabe",
            "signer_pubkey": "pubkey_a",
            "signature_hex": "sig_a"
        });
        let req = json_request("POST", "/proposals", Some(body2));
        app.oneshot(req).await.unwrap();

        // List
        let app = router(state);
        let req = json_request("GET", "/proposals", None);
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_json(resp).await;
        assert_eq!(body["proposals"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_list_proposals_filter_by_status() {
        let config = crate::config::Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 0,
        };
        let state = AppState::new(config);

        let app = router(state.clone());
        let req = json_request("POST", "/proposals", Some(create_body()));
        app.oneshot(req).await.unwrap();

        // Filter pending
        let app = router(state.clone());
        let req = json_request("GET", "/proposals?status=pending", None);
        let resp = app.oneshot(req).await.unwrap();
        let body = response_json(resp).await;
        assert_eq!(body["proposals"].as_array().unwrap().len(), 1);

        // Filter approved — should be empty
        let app = router(state);
        let req = json_request("GET", "/proposals?status=approved", None);
        let resp = app.oneshot(req).await.unwrap();
        let body = response_json(resp).await;
        assert_eq!(body["proposals"].as_array().unwrap().len(), 0);
    }

    // ─── get_proposal ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_proposal_happy_path() {
        let config = crate::config::Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 0,
        };
        let state = AppState::new(config);

        let app = router(state.clone());
        let req = json_request("POST", "/proposals", Some(create_body()));
        let resp = app.oneshot(req).await.unwrap();
        let created = response_json(resp).await;
        let action_id = created["action_id"].as_str().unwrap();

        let app = router(state);
        let req = json_request("GET", &format!("/proposals/{action_id}"), None);
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_json(resp).await;
        assert_eq!(body["seq_no"], 1);
        assert_eq!(body["status"], "pending");
    }

    #[tokio::test]
    async fn test_get_proposal_not_found() {
        let app = test_app();
        let req = json_request("GET", "/proposals/nonexistent", None);
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ─── approve_action ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_approve_action_happy_path() {
        let config = crate::config::Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 0,
        };
        let state = AppState::new(config);

        let app = router(state.clone());
        let req = json_request("POST", "/proposals", Some(create_body()));
        let resp = app.oneshot(req).await.unwrap();
        let created = response_json(resp).await;
        let action_id = created["action_id"].as_str().unwrap();

        let app = router(state);
        let sig_body = json!({
            "signer_pubkey": "pubkey_b",
            "signature_hex": "sig_b"
        });
        let req = json_request(
            "POST",
            &format!("/proposals/{action_id}/approve"),
            Some(sig_body),
        );
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = response_json(resp).await;
        assert_eq!(body["proposal"]["signatures"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_approve_action_duplicate_signer_rejected() {
        let config = crate::config::Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 0,
        };
        let state = AppState::new(config);

        let app = router(state.clone());
        let req = json_request("POST", "/proposals", Some(create_body()));
        let resp = app.oneshot(req).await.unwrap();
        let created = response_json(resp).await;
        let action_id = created["action_id"].as_str().unwrap();

        let app = router(state);
        let sig_body = json!({
            "signer_pubkey": "pubkey_a",
            "signature_hex": "sig_a_again"
        });
        let req = json_request(
            "POST",
            &format!("/proposals/{action_id}/approve"),
            Some(sig_body),
        );
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_approve_action_nonexistent_proposal() {
        let app = test_app();
        let sig_body = json!({
            "signer_pubkey": "pubkey_a",
            "signature_hex": "sig_a"
        });
        let req = json_request("POST", "/proposals/nonexistent/approve", Some(sig_body));
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
