use crate::state::AppState;
use axum::{
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};

pub mod auth;
pub mod auth_session;
pub mod proposals;

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        // Auth
        .route("/auth/challenge", post(auth::auth_challenge))
        .route("/auth/verify", post(auth::auth_verify))
        .route("/auth/logout", post(auth::auth_logout))
        // Proposals
        .route("/proposals/next-seq-no", get(proposals::get_next_seq_no))
        .route("/proposals", get(proposals::list_proposals))
        .route("/proposals", post(proposals::create_proposal))
        .route("/proposals/:action_id", get(proposals::get_proposal))
        .route(
            "/proposals/:action_id/approve",
            post(proposals::approve_action),
        )
        .route(
            "/proposals/:action_id/broadcast/prepare",
            post(proposals::prepare_broadcast),
        )
        .route(
            "/proposals/:action_id/broadcast",
            post(proposals::execute_broadcast),
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
    use std::sync::Arc;
    use tower::util::ServiceExt;

    const SIGNER_A_SK: &str = "0000000000000000000000000000000000000000000000000000000000000001";
    const SIGNER_A_PK: &str = "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";
    const SIGNER_B_PK: &str = "02c6047f9441ed7d6d3045406e95c07cd85a1a3f1f3ff2b4f6f3f5b4f0c709ee5";
    const NON_MEMBER_PK: &str =
        "03aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn test_app_with_rpc_url(rpc_url: &str) -> Router {
        use crate::infrastructure::{
            bitcoin_rpc::HttpBitcoinRpcClient, memory_repo::InMemoryProposalRepository,
        };
        use bitcoin::{
            key::UntweakedKeypair,
            secp256k1::{SecretKey, SECP256K1},
            Network,
        };
        use strata_l1_txfmt::MagicBytes;

        let repo = Arc::new(InMemoryProposalRepository::new());
        let sk = SecretKey::from_slice(&[1u8; 32]).unwrap();
        let keypair = UntweakedKeypair::from_secret_key(SECP256K1, &sk);
        let btc_client = Arc::new(HttpBitcoinRpcClient::new(
            "http://127.0.0.1:18443",
            None,
            "user",
            "pass",
        ));
        router(AppState::new(
            repo,
            rpc_url.to_string(),
            120_000,
            240_000,
            btc_client,
            keypair,
            5_000,
            600_000,
            MagicBytes::new([0x41, 0x4c, 0x50, 0x4e]),
            Network::Regtest,
        ))
    }

    fn test_app() -> Router {
        test_app_with_rpc_url("mock://asm-membership")
    }

    fn json_request(
        method: &str,
        uri: &str,
        body: Option<Value>,
        bearer: Option<&str>,
    ) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(token) = bearer {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }

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

    fn create_body(signer: &str) -> Value {
        json!({
            "seq_no": 1,
            "action_hex": "deadbeef",
            "signer_pubkey": signer,
            "signature_hex": "sig_a"
        })
    }

    fn sign_digest(secret_key_hex: &str, challenge_hex: &str) -> String {
        use bitcoin::secp256k1::{Message, Secp256k1, SecretKey};
        let sk_bytes = hex::decode(secret_key_hex).unwrap();
        let sk = SecretKey::from_slice(&sk_bytes).unwrap();
        let digest = hex::decode(challenge_hex).unwrap();
        let msg = Message::from_digest_slice(&digest).unwrap();
        let sig = Secp256k1::new().sign_ecdsa(&msg, &sk);
        hex::encode(sig.serialize_compact())
    }

    async fn login(app: Router, signer_sk: &str, signer_pk: &str) -> String {
        let req = json_request(
            "POST",
            "/auth/challenge",
            Some(json!({ "authority": "strata_admin" })),
            None,
        );
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let challenge = response_json(resp).await;
        let challenge_id = challenge["challenge_id"].as_str().unwrap();
        let challenge_hex = challenge["challenge_hex"].as_str().unwrap();
        let signature_hex = sign_digest(signer_sk, challenge_hex);

        let req = json_request(
            "POST",
            "/auth/verify",
            Some(json!({
                "challenge_id": challenge_id,
                "signer_pubkey": signer_pk,
                "signature_hex": signature_hex,
                "signature_format": "p2wpkh-tx-binding"
            })),
            None,
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = response_json(resp).await;
        body["token"].as_str().unwrap().to_string()
    }

    #[tokio::test]
    async fn test_auth_verify_non_member_rejected() {
        let app = test_app();
        let req = json_request(
            "POST",
            "/auth/challenge",
            Some(json!({ "authority": "strata_admin" })),
            None,
        );
        let resp = app.clone().oneshot(req).await.unwrap();
        let challenge = response_json(resp).await;
        let req = json_request(
            "POST",
            "/auth/verify",
            Some(json!({
                "challenge_id": challenge["challenge_id"],
                "signer_pubkey": NON_MEMBER_PK,
                "signature_hex": "00",
                "signature_format": "p2wpkh-tx-binding"
            })),
            None,
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_verify_signer_a_fixture_accepted() {
        let app = test_app();
        let token = login(app, SIGNER_A_SK, SIGNER_A_PK).await;
        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn test_auth_verify_unmapped_authority_is_fail_closed() {
        let app = test_app();
        let req = json_request(
            "POST",
            "/auth/challenge",
            Some(json!({ "authority": "alpen_admin" })),
            None,
        );
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let challenge = response_json(resp).await;
        let challenge_id = challenge["challenge_id"].as_str().unwrap();
        let challenge_hex = challenge["challenge_hex"].as_str().unwrap();
        let signature_hex = sign_digest(SIGNER_A_SK, challenge_hex);

        let req = json_request(
            "POST",
            "/auth/verify",
            Some(json!({
                "challenge_id": challenge_id,
                "signer_pubkey": SIGNER_A_PK,
                "signature_hex": signature_hex,
                "signature_format": "p2wpkh-tx-binding"
            })),
            None,
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_auth_verify_fails_closed_when_rpc_unavailable() {
        let app = test_app_with_rpc_url("http://127.0.0.1:1");
        let req = json_request(
            "POST",
            "/auth/challenge",
            Some(json!({ "authority": "strata_admin" })),
            None,
        );
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let challenge = response_json(resp).await;
        let challenge_id = challenge["challenge_id"].as_str().unwrap();
        let challenge_hex = challenge["challenge_hex"].as_str().unwrap();
        let signature_hex = sign_digest(SIGNER_A_SK, challenge_hex);

        let req = json_request(
            "POST",
            "/auth/verify",
            Some(json!({
                "challenge_id": challenge_id,
                "signer_pubkey": SIGNER_A_PK,
                "signature_hex": signature_hex,
                "signature_format": "p2wpkh-tx-binding"
            })),
            None,
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_proposal_requires_bearer() {
        let app = test_app();
        let req = json_request("POST", "/proposals", Some(create_body(SIGNER_A_PK)), None);
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_create_and_get_with_bearer() {
        let app = test_app();
        let token = login(app.clone(), SIGNER_A_SK, SIGNER_A_PK).await;

        let req = json_request(
            "POST",
            "/proposals",
            Some(create_body(SIGNER_A_PK)),
            Some(&token),
        );
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let created = response_json(resp).await;
        let action_id = created["action_id"].as_str().unwrap().to_string();

        let req = json_request(
            "GET",
            &format!("/proposals/{action_id}"),
            None,
            Some(&token),
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_signer_mismatch_rejected() {
        let app = test_app();
        let token = login(app.clone(), SIGNER_A_SK, SIGNER_A_PK).await;
        let req = json_request(
            "POST",
            "/proposals",
            Some(create_body(SIGNER_B_PK)),
            Some(&token),
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_get_proposal_wrong_authority_returns_unauthorized() {
        use crate::application::traits::ProposalRepository;
        use crate::domain::authority::Authority;
        use crate::domain::proposal::{
            ActionId, BroadcastStatus, Proposal, ProposalSignature, ProposalStatus,
        };
        use crate::infrastructure::memory_repo::InMemoryProposalRepository;

        let repo = Arc::new(InMemoryProposalRepository::new());
        let action_id = "cross_auth_action".to_string();
        repo.save_proposal(Proposal {
            action_id: ActionId(action_id.clone()),
            seq_no: 1,
            authority: Authority::AlpenAdmin,
            status: ProposalStatus::Pending,
            required_signatures: 2,
            action_hex: "deadbeef".to_string(),
            signatures: vec![ProposalSignature {
                signer_pubkey: SIGNER_A_PK.to_string(),
                signature_hex: "sig_a".to_string(),
            }],
            broadcast_status: BroadcastStatus::default(),
            commit_txid: None,
            reveal_txid: None,
            broadcast_error: None,
        })
        .await
        .unwrap();

        let app = {
            use crate::infrastructure::bitcoin_rpc::HttpBitcoinRpcClient;
            use bitcoin::{
                key::UntweakedKeypair,
                secp256k1::{SecretKey, SECP256K1},
                Network,
            };
            use strata_l1_txfmt::MagicBytes;

            let sk = SecretKey::from_slice(&[1u8; 32]).unwrap();
            let keypair = UntweakedKeypair::from_secret_key(SECP256K1, &sk);
            let btc_client = Arc::new(HttpBitcoinRpcClient::new(
                "http://127.0.0.1:18443",
                None,
                "user",
                "pass",
            ));
            router(AppState::new(
                repo,
                "mock://asm-membership".to_string(),
                120_000,
                240_000,
                btc_client,
                keypair,
                5_000,
                600_000,
                MagicBytes::new([0x41, 0x4c, 0x50, 0x4e]),
                Network::Regtest,
            ))
        };

        let token = login(app.clone(), SIGNER_A_SK, SIGNER_A_PK).await;
        let req = json_request(
            "GET",
            &format!("/proposals/{action_id}"),
            None,
            Some(&token),
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_approve_with_invalid_token_rejected() {
        let app = test_app();
        let token = login(app.clone(), SIGNER_A_SK, SIGNER_A_PK).await;
        let req = json_request(
            "POST",
            "/proposals",
            Some(create_body(SIGNER_A_PK)),
            Some(&token),
        );
        let resp = app.clone().oneshot(req).await.unwrap();
        let created = response_json(resp).await;
        let action_id = created["action_id"].as_str().unwrap();

        let req = json_request(
            "POST",
            &format!("/proposals/{action_id}/approve"),
            Some(json!({
                "signer_pubkey": SIGNER_B_PK,
                "signature_hex": "sig_b"
            })),
            Some("invalid-token"),
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
