//! E2E integration test: Propose → Sign → Verify flow.
//!
//! Exercises the real desktop application layer (`proposals.rs`) making real HTTP
//! calls to a real orchestrator server running as a subprocess.
//!
//! Happy path: authenticate → create → get → approve → get → verify_threshold

use std::net::TcpListener;
use std::num::NonZeroU8;
use std::process::{Child, Command};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bitcoin::secp256k1::{Message, PublicKey, SecretKey, SECP256K1};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use desktop_app::application::proposals;
use desktop_app::domain::action::{Action, CompressedPubKey, MultisigUpdate};
use desktop_app::domain::authority::Authority;
use desktop_app::domain::proposal::Signature;
use desktop_app::infrastructure::action_codec;
use desktop_app::infrastructure::orchestrator_client::HttpOrchestratorClient;
use desktop_app::infrastructure::signing;

// ─── Mock ASM RPC ──────────────────────────────────────────────────────────

/// Starts a minimal HTTP server that responds to any POST with a JSON-RPC result
/// containing the given signer pubkey in the Strata administrator key set.
/// Returns the URL of the mock server.
async fn start_mock_asm_rpc(signer_pubkey_hex: String) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock ASM RPC");
    let addr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let pubkey = signer_pubkey_hex.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf).await;
                let body = format!(
                    r#"{{"jsonrpc":"2.0","id":1,"result":{{"authorities":{{"strata_administrator":{{"config":{{"keys":["{pubkey}"]}}}}}}}}}}"#
                );
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });

    format!("http://127.0.0.1:{}", addr.port())
}

// ─── Auth helpers ───────────────────────────────────────────────────────────

/// Authenticate with the backend and return (session_token, ephemeral_sk).
async fn authenticate(base_url: &str, signer_sk: &SecretKey) -> (String, SecretKey) {
    let signer_pk = PublicKey::from_secret_key(SECP256K1, signer_sk);
    let signer_pk_hex = hex::encode(signer_pk.serialize());

    let eph_sk = SecretKey::new(&mut OsRng);
    let eph_pk = PublicKey::from_secret_key(SECP256K1, &eph_sk);
    let eph_pk_hex = hex::encode(eph_pk.serialize());

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let expiry_secs = now + 3600u64;

    let mut nonce_bytes = [0u8; 32];
    use rand::RngCore;
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce_hex = hex::encode(nonce_bytes);

    let eph_pk_bytes = eph_pk.serialize();
    let mut hasher = Sha256::new();
    hasher.update(b"alpen-multisig:auth-v1");
    hasher.update(b"strata_admin");
    hasher.update(&eph_pk_bytes);
    hasher.update(nonce_bytes);
    hasher.update(expiry_secs.to_be_bytes());
    let hash = hasher.finalize();

    let msg = Message::from_digest_slice(&hash).unwrap();
    let sig = SECP256K1.sign_ecdsa(&msg, signer_sk);
    let sig_hex = hex::encode(sig.serialize_compact());

    let resp: serde_json::Value = reqwest::Client::new()
        .post(format!("{base_url}/auth"))
        .json(&serde_json::json!({
            "authority": "strata_admin",
            "signer_pubkey": signer_pk_hex,
            "ephemeral_pubkey": eph_pk_hex,
            "nonce": nonce_hex,
            "expiry_secs": expiry_secs,
            "signature": sig_hex,
        }))
        .send()
        .await
        .expect("auth POST failed")
        .json()
        .await
        .expect("auth response parse failed");

    let token = resp["session_token"]
        .as_str()
        .expect("session_token in response")
        .to_string();

    (token, eph_sk)
}

// ─── Test server ───────────────────────────────────────────────────────────

struct TestServer {
    child: Child,
    base_url: String,
}

impl TestServer {
    /// Start the orchestrator as a subprocess on a random available port.
    async fn start(mock_rpc_url: &str) -> Self {
        let port = find_available_port();
        let binary = orchestrator_binary();

        let child = Command::new(&binary)
            .env("SERVER_HOST", "127.0.0.1")
            .env("SERVER_PORT", port.to_string())
            .env("STRATA_ADMIN_STATE_RPC_URL", mock_rpc_url)
            .env("RUST_LOG", "warn")
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
                .args(["build", "-p", "orchestator-be"])
                .status()
                .expect("cargo build failed");
            assert!(status.success(), "cargo build -p orchestator-be failed");

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

fn generate_keypair() -> (String, String) {
    let sk = SecretKey::new(&mut OsRng);
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

// ─── Tests ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_e2e_propose_approve_verify() {
    // 1. Generate signer keypair and start mock ASM RPC claiming this signer is in the set.
    let signer_sk = SecretKey::new(&mut OsRng);
    let signer_pk = PublicKey::from_secret_key(SECP256K1, &signer_sk);
    let signer_pk_hex = hex::encode(signer_pk.serialize());

    let mock_rpc_url = start_mock_asm_rpc(signer_pk_hex).await;

    // 2. Start orchestrator subprocess with mock RPC URL.
    let server = TestServer::start(&mock_rpc_url).await;

    // 3. Authenticate to obtain a session token + ephemeral key.
    let (session_token, eph_sk) = authenticate(&server.base_url, &signer_sk).await;
    let client =
        HttpOrchestratorClient::new_authenticated(server.base_url.clone(), session_token, eph_sk);

    // 4. Signer A: generate keypair, build action, sign.
    let (sk_a, _pk_a) = generate_keypair();
    let action = build_demo_action();
    let action_hex = action_codec::encode_hex(&action).expect("encode action");
    let seq_no = 1u64;
    let sig_a = sign_action(&sk_a, seq_no, &action_hex);

    // 5. Create proposal via desktop application layer.
    let created = proposals::create_update_action(
        &client,
        Authority::StrataAdmin,
        action_hex.as_str(),
        seq_no,
        &sig_a,
    )
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

    // 6. Get proposal — verify it was persisted correctly.
    let fetched = proposals::get_update_action(&client, action_id)
        .await
        .expect("get_update_action should succeed");

    assert_eq!(fetched.action_id, *action_id);
    assert_eq!(fetched.seq_no, seq_no);
    assert_eq!(fetched.authority, Authority::StrataAdmin);
    assert_eq!(fetched.action_hex, action_hex);
    assert_eq!(fetched.signatures.len(), 1);

    // 7. Signer B: generate keypair, sign the same action.
    let (sk_b, _pk_b) = generate_keypair();
    let sig_b = sign_action(&sk_b, seq_no, &action_hex);

    // 8. Approve proposal via desktop application layer.
    let approved = proposals::approve_action(&client, action_id, &sig_b)
        .await
        .expect("approve_action should succeed");

    assert_eq!(approved.action_id, *action_id);
    assert_eq!(approved.signatures.len(), 2);

    // 9. Get proposal again — verify both signatures are persisted.
    let final_state = proposals::get_update_action(&client, action_id)
        .await
        .expect("get_update_action after approve should succeed");

    assert_eq!(final_state.signatures.len(), 2);
    assert_eq!(final_state.action_hex, action_hex);

    // 10. Verify threshold: both signatures are valid.
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

    let verify = signing::verify_threshold(&pubkeys, 2, &sigs, &sighash.sighash_hex)
        .expect("verify_threshold should succeed");

    assert!(verify.valid, "threshold verification must pass");
}
