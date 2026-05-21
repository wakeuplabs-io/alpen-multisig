//! E2E integration test: Propose → Sign → Verify flow.
//!
//! Exercises the real desktop application layer (`proposals.rs`) making real HTTP
//! calls to a real orchestrator server running as a subprocess.
//!
//! Happy path: create → get → approve → get → verify_threshold

use std::net::TcpListener;
use std::num::NonZeroU8;
use std::process::{Child, Command};
use std::time::Duration;

use bitcoin::secp256k1::{PublicKey, SecretKey, SECP256K1};

use desktop_app::application::orchestrator_client::{
    CompleteOrchestratorAuthRequest, OrchestratorClient, StartOrchestratorAuthRequest,
};
use desktop_app::application::proposals;
use desktop_app::domain::action::{Action, CompressedPubKey, MultisigUpdate};
use desktop_app::domain::authority::Authority;
use desktop_app::domain::proposal::Signature;
use desktop_app::infrastructure::action_codec;
use desktop_app::infrastructure::orchestrator_client::HttpOrchestratorClient;
use desktop_app::infrastructure::signing;

// ─── Test server ───────────────────────────────────────────────────────────

struct TestServer {
    child: Child,
    base_url: String,
}

impl TestServer {
    /// Start the orchestrator as a subprocess on a random available port.
    async fn start() -> Self {
        let port = find_available_port();
        let binary = orchestrator_binary();

        let child = Command::new(&binary)
            .env("SERVER_HOST", "127.0.0.1")
            .env("SERVER_PORT", port.to_string())
            .env("RUST_LOG", "warn")
            .env("STRATA_ADMIN_STATE_RPC_URL", "mock://asm-membership")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("Failed to start orchestrator at {binary:?}: {e}"));

        let base_url = format!("http://127.0.0.1:{port}/api/v1");

        let server = Self { child, base_url };
        server.wait_for_health().await;
        server
    }

    /// Poll the health endpoint until it responds.
    async fn wait_for_health(&self) {
        let client = reqwest::Client::new();
        let health_url = format!("{}/health", self.base_url);

        for i in 0..50 {
            if i > 0 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            if client.get(&health_url).send().await.is_ok() {
                return;
            }
        }
        panic!("Orchestrator did not become healthy within 5 seconds");
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn find_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind to random port")
        .local_addr()
        .expect("local addr")
        .port()
}

static BINARY_PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Build the orchestrator binary once and return the path to the executable.
fn orchestrator_binary() -> String {
    BINARY_PATH
        .get_or_init(|| {
            let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap();

            let status = Command::new("cargo")
                .current_dir(workspace_root)
                .args(["build", "-p", "orchestrator-be"])
                .status()
                .expect("cargo build failed");
            assert!(status.success(), "cargo build -p orchestrator-be failed");

            let output = Command::new("cargo")
                .current_dir(workspace_root)
                .args(["metadata", "--format-version", "1", "--no-deps"])
                .output()
                .expect("cargo metadata failed");
            let metadata: serde_json::Value =
                serde_json::from_slice(&output.stdout).expect("parse metadata");
            let target_dir = metadata["target_directory"]
                .as_str()
                .expect("target_directory");
            format!("{target_dir}/debug/server")
        })
        .clone()
}

// ─── Crypto helpers ────────────────────────────────────────────────────────

fn keypair_from_fixed_secret(secret_key_bytes: [u8; 32]) -> (String, String) {
    let sk = SecretKey::from_slice(&secret_key_bytes).expect("valid fixed key");
    let pk = PublicKey::from_secret_key(SECP256K1, &sk);
    (hex::encode(sk.secret_bytes()), hex::encode(pk.serialize()))
}

fn build_demo_action() -> Action {
    let demo_bytes = [0x42u8; 32];
    let demo_sk = SecretKey::from_slice(&demo_bytes).expect("valid fixed key");
    let new_signer_pk = PublicKey::from_secret_key(SECP256K1, &demo_sk);
    let new_signer = CompressedPubKey::new(new_signer_pk.serialize());
    Action::MultisigUpdate(MultisigUpdate {
        role: Authority::StrataAdmin,
        add_keys: vec![new_signer],
        remove_keys: vec![],
        new_threshold: NonZeroU8::new(2).expect("non-zero"),
    })
}

fn sign_action(secret_key_hex: &str, seq_no: u64, action_hex: &str) -> Signature {
    let sighash = signing::compute_sighash(seq_no, action_hex).expect("sighash ok");
    let sig = signing::sign_sighash(secret_key_hex, &sighash.sighash_hex).expect("sign ok");
    Signature {
        signer_pubkey: sig.public_key_hex,
        signature_hex: sig.signature_hex,
    }
}

fn sign_auth_challenge(secret_key_hex: &str, challenge_hex: &str) -> String {
    let sk_bytes = hex::decode(secret_key_hex).expect("secret key hex");
    let sk = SecretKey::from_slice(&sk_bytes).expect("valid secret key");
    let digest = hex::decode(challenge_hex).expect("challenge hex");
    let msg = bitcoin::secp256k1::Message::from_digest_slice(&digest).expect("challenge digest");
    let sig = SECP256K1.sign_ecdsa(&msg, &sk);
    hex::encode(sig.serialize_compact())
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_e2e_propose_approve_verify() {
    // 1. Start orchestrator subprocess
    let server = TestServer::start().await;
    // 2. Signer A: generate keypair, build action, sign
    let mut sk_a_bytes = [0u8; 32];
    sk_a_bytes[31] = 1;
    let (sk_a, pk_a) = keypair_from_fixed_secret(sk_a_bytes);
    let auth_client = HttpOrchestratorClient::new(server.base_url.clone());
    let challenge = auth_client
        .auth_challenge(StartOrchestratorAuthRequest {
            authority: "strata_admin".to_string(),
        })
        .await
        .expect("auth challenge should succeed");
    let auth_sig = sign_auth_challenge(&sk_a, &challenge.challenge_hex);
    let session = auth_client
        .auth_verify(CompleteOrchestratorAuthRequest {
            challenge_id: challenge.challenge_id,
            signer_pubkey: pk_a,
            signature_hex: auth_sig,
            signature_format: "p2wpkh-tx-binding".to_string(),
        })
        .await
        .expect("auth verify should succeed");
    let client =
        HttpOrchestratorClient::new(server.base_url.clone()).with_bearer_token(session.token);
    let action = build_demo_action();
    let action_hex = action_codec::encode_hex(&action).expect("encode action");
    let seq_no = 1u64;
    let sig_a = sign_action(&sk_a, seq_no, &action_hex);

    // 3. Create proposal via desktop application layer
    let created = proposals::create_update_action(&client, action_hex.as_str(), seq_no, &sig_a)
        .await
        .expect("create_update_action should succeed");

    assert_eq!(created.status, "pending");
    assert_eq!(created.seq_no, seq_no);
    assert_eq!(created.authority, Authority::StrataAdmin);
    assert_eq!(created.action_hex, action_hex);
    assert_eq!(created.signatures.len(), 1);
    assert_eq!(created.signatures[0].signer_pubkey, sig_a.signer_pubkey);
    assert_eq!(created.signatures[0].signature_hex, sig_a.signature_hex);

    let action_id = &created.action_id;

    // 4. Get proposal — verify it was persisted correctly
    let fetched = proposals::get_update_action(&client, action_id)
        .await
        .expect("get_update_action should succeed");

    assert_eq!(fetched.action_id, *action_id);
    assert_eq!(fetched.seq_no, seq_no);
    assert_eq!(fetched.authority, Authority::StrataAdmin);
    assert_eq!(fetched.action_hex, action_hex);
    assert_eq!(fetched.signatures.len(), 1);

    // 5. Get proposal again — verify created signature remains persisted
    let final_state = proposals::get_update_action(&client, action_id)
        .await
        .expect("get_update_action second call should succeed");

    assert_eq!(final_state.signatures.len(), 1);
    assert_eq!(final_state.action_hex, action_hex);

    // 6. Verify threshold: initial signature is valid
    let sighash = signing::compute_sighash(seq_no, &action_hex).expect("sighash ok");

    let pubkeys: Vec<String> = final_state
        .signatures
        .iter()
        .map(|s| s.signer_pubkey.clone())
        .collect();
    let sigs: Vec<String> = final_state
        .signatures
        .iter()
        .map(|s| s.signature_hex.clone())
        .collect();

    let verify = signing::verify_threshold(&pubkeys, 1, &sigs, &sighash.sighash_hex)
        .expect("verify_threshold should succeed");

    assert!(verify.valid, "threshold verification must pass");

    // 7. Signer B approves → quorum (2/2) → auto-transitions to approved
    let mut sk_b_bytes = [0u8; 32];
    sk_b_bytes[31] = 2;
    let (sk_b, pk_b) = keypair_from_fixed_secret(sk_b_bytes);

    let challenge_b = auth_client
        .auth_challenge(StartOrchestratorAuthRequest {
            authority: "strata_admin".to_string(),
        })
        .await
        .expect("auth challenge B should succeed");
    let auth_sig_b = sign_auth_challenge(&sk_b, &challenge_b.challenge_hex);
    let session_b = auth_client
        .auth_verify(CompleteOrchestratorAuthRequest {
            challenge_id: challenge_b.challenge_id,
            signer_pubkey: pk_b,
            signature_hex: auth_sig_b,
            signature_format: "p2wpkh-tx-binding".to_string(),
        })
        .await
        .expect("auth verify B should succeed");
    let client_b =
        HttpOrchestratorClient::new(server.base_url.clone()).with_bearer_token(session_b.token);

    let sig_b = sign_action(&sk_b, seq_no, &action_hex);
    let approved = proposals::approve_action(&client_b, action_id, &sig_b)
        .await
        .expect("approve_action signer B should succeed");

    assert_eq!(
        approved.status, "approved",
        "proposal must be approved after quorum reached"
    );
    assert_eq!(
        approved.signatures.len(),
        2,
        "two signatures must be stored"
    );
}
