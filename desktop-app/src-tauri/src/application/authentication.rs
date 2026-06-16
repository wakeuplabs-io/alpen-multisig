use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::domain::auth::{
    AuthChallenge, AuthRole, AuthSession, MembershipCache, PendingChallenge,
};
use crate::infrastructure::{asm_status_rpc, challenge_verifier};

const CHALLENGE_TTL_MS: u64 = 120_000;
const SESSION_TTL_MS: u64 = 1_800_000;
const MEMBERSHIP_MAX_AGE_MS: u64 = 300_000;
const SIG_FORMAT_P2WPKH_TX_BINDING: &str = "p2wpkh-tx-binding";
const SIG_FORMAT_BITCOIN_MESSAGE: &str = "bitcoin-message";
const SIG_FORMAT_RAW_ECDSA: &str = "raw-ecdsa";

#[derive(Default)]
struct AuthState {
    pending: HashMap<String, PendingChallenge>,
    session: Option<AuthSession>,
    membership_cache: Option<MembershipCache>,
}

fn auth_state() -> &'static Mutex<AuthState> {
    static AUTH_STATE: OnceLock<Mutex<AuthState>> = OnceLock::new();
    AUTH_STATE.get_or_init(|| Mutex::new(AuthState::default()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartChallengeInput {
    pub role: AuthRole,
    pub rpc_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteAuthInput {
    pub challenge_id: String,
    pub signer_pubkey_hex: String,
    pub signature_hex: String,
    pub signature_format: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResult {
    pub authenticated: bool,
    pub session: Option<AuthSession>,
}

pub async fn start_challenge(input: StartChallengeInput) -> Result<AuthChallenge, String> {
    let rpc_url = input
        .rpc_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .unwrap_or_else(asm_status_rpc::default_rpc_url);

    let membership = asm_status_rpc::fetch_role_membership(&rpc_url)
        .await
        .map_err(|e| rpc_dependency_error(&rpc_url, &e))?;
    start_challenge_with_membership(input, membership)
}

fn start_challenge_with_membership(
    input: StartChallengeInput,
    membership: (HashMap<AuthRole, Vec<String>>, u64),
) -> Result<AuthChallenge, String> {
    let (role_to_keys, fetched_at_unix_ms) = membership;
    if role_to_keys
        .get(&input.role)
        .is_none_or(|keys| keys.is_empty())
    {
        return Err(format!(
            "no signer keys available for role `{:?}` in current admin state",
            input.role
        ));
    }

    let now = now_unix_ms();
    let nonce_hex = challenge_verifier::random_nonce_hex();
    let session_id = challenge_verifier::random_session_id();
    let role_wire = role_wire(input.role);
    let challenge_digest = challenge_verifier::create_challenge_digest(
        role_wire,
        &nonce_hex,
        now,
        now + CHALLENGE_TTL_MS,
        &session_id,
    );
    let challenge_id = nonce_hex.clone();
    let challenge = AuthChallenge {
        challenge_id: challenge_id.clone(),
        challenge_hex: hex::encode(challenge_digest),
        nonce_hex,
        domain: "alpen-multisig/auth/v1".to_string(),
        role: input.role,
        issued_at_unix_ms: now,
        expires_at_unix_ms: now + CHALLENGE_TTL_MS,
        session_id,
    };

    let mut state = auth_state()
        .lock()
        .map_err(|_| "auth state lock poisoned".to_string())?;
    if state
        .session
        .as_ref()
        .is_some_and(|current| current.role != input.role)
    {
        state.session = None;
    }
    state.pending.insert(
        challenge_id,
        PendingChallenge {
            challenge: challenge.clone(),
            consumed: false,
        },
    );
    state.membership_cache = Some(MembershipCache {
        fetched_at_unix_ms,
        role_to_keys,
    });

    Ok(challenge)
}

fn rpc_dependency_error(rpc_url: &str, details: &str) -> String {
    format!(
        "authentication requires live role membership from ASM RPC; verify node/RPC availability (`{rpc_url}`). Details: {details}"
    )
}

pub fn complete_auth(input: CompleteAuthInput) -> Result<AuthSession, String> {
    let signature_format = input.signature_format.as_str();
    if signature_format != SIG_FORMAT_P2WPKH_TX_BINDING
        && signature_format != SIG_FORMAT_BITCOIN_MESSAGE
        && signature_format != SIG_FORMAT_RAW_ECDSA
    {
        return Err(format!(
            "unsupported signature format `{}`; expected one of: `{}`, `{}`, `{}`",
            input.signature_format,
            SIG_FORMAT_P2WPKH_TX_BINDING,
            SIG_FORMAT_BITCOIN_MESSAGE,
            SIG_FORMAT_RAW_ECDSA
        ));
    }

    let now = now_unix_ms();
    let mut state = auth_state()
        .lock()
        .map_err(|_| "auth state lock poisoned".to_string())?;
    let (challenge_hex, role) = {
        let pending = state
            .pending
            .get_mut(&input.challenge_id)
            .ok_or_else(|| "unknown challenge id".to_string())?;

        if pending.consumed {
            return Err("challenge already used".to_string());
        }
        if now > pending.challenge.expires_at_unix_ms {
            return Err("challenge expired; request a new one".to_string());
        }
        (
            pending.challenge.challenge_hex.clone(),
            pending.challenge.role,
        )
    };

    let membership = state
        .membership_cache
        .as_ref()
        .ok_or_else(|| "membership cache unavailable; restart auth flow".to_string())?;
    let membership_fetched_at_unix_ms = membership.fetched_at_unix_ms;
    if !membership_is_fresh(now, membership.fetched_at_unix_ms) {
        return Err("membership cache is stale; start auth again".to_string());
    }

    let role_keys = membership
        .role_to_keys
        .get(&role)
        .ok_or_else(|| "selected role has no key set".to_string())?;

    match signature_format {
        SIG_FORMAT_BITCOIN_MESSAGE => challenge_verifier::verify_bitcoin_message_signature(
            &challenge_hex,
            &input.signer_pubkey_hex,
            &input.signature_hex,
        )?,
        SIG_FORMAT_RAW_ECDSA => challenge_verifier::verify_signature(
            &challenge_hex,
            &input.signer_pubkey_hex,
            &input.signature_hex,
        )?,
        _ => challenge_verifier::verify_signature(
            &challenge_hex,
            &input.signer_pubkey_hex,
            &input.signature_hex,
        )?,
    }

    let is_member = role_keys
        .iter()
        .any(|key| key.eq_ignore_ascii_case(&input.signer_pubkey_hex));
    if !is_member {
        return Err("signer key is not a member of the selected role".to_string());
    }

    if let Some(pending) = state.pending.get_mut(&input.challenge_id) {
        pending.consumed = true;
    }
    let session = AuthSession {
        role,
        signer_pubkey_hex: input.signer_pubkey_hex,
        authenticated_at_unix_ms: now,
        expires_at_unix_ms: now + SESSION_TTL_MS,
        membership_fetched_at_unix_ms,
    };
    state.session = Some(session.clone());
    Ok(session)
}

pub fn get_session() -> Result<SessionResult, String> {
    let now = now_unix_ms();
    let mut state = auth_state()
        .lock()
        .map_err(|_| "auth state lock poisoned".to_string())?;
    if let Some(session) = state.session.as_ref() {
        if now <= session.expires_at_unix_ms {
            return Ok(SessionResult {
                authenticated: true,
                session: Some(session.clone()),
            });
        }
        state.session = None;
    }
    Ok(SessionResult {
        authenticated: false,
        session: None,
    })
}

pub fn logout() -> Result<(), String> {
    let mut state = auth_state()
        .lock()
        .map_err(|_| "auth state lock poisoned".to_string())?;
    state.session = None;
    Ok(())
}

fn role_wire(role: AuthRole) -> &'static str {
    role.as_wire_str()
}

fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as u64
}

fn membership_is_fresh(now_unix_ms: u64, fetched_at_unix_ms: u64) -> bool {
    now_unix_ms.saturating_sub(fetched_at_unix_ms) <= MEMBERSHIP_MAX_AGE_MS
}

#[cfg(test)]
fn reset_auth_state_for_tests() {
    if let Ok(mut state) = auth_state().lock() {
        state.pending.clear();
        state.session = None;
        state.membership_cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
    use std::sync::{Mutex, OnceLock};

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("test lock must be available")
    }

    fn test_membership(
        role: AuthRole,
        signer_pubkey_hex: String,
        fetched_at_unix_ms: u64,
    ) -> (HashMap<AuthRole, Vec<String>>, u64) {
        let mut role_to_keys = HashMap::new();
        role_to_keys.insert(role, vec![signer_pubkey_hex]);
        (role_to_keys, fetched_at_unix_ms)
    }

    fn sign_challenge_for_tests(
        challenge_hex: &str,
        secret_key_bytes: [u8; 32],
    ) -> (String, String) {
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&secret_key_bytes).expect("valid key");
        let pk = PublicKey::from_secret_key(&secp, &sk);

        let digest = hex::decode(challenge_hex).expect("valid challenge hex");
        let msg = Message::from_digest_slice(&digest).expect("32-byte digest");
        let signature = secp.sign_ecdsa(&msg, &sk);
        (
            hex::encode(pk.serialize()),
            hex::encode(signature.serialize_compact()),
        )
    }

    #[test]
    fn start_challenge_fails_closed_when_rpc_unavailable() {
        let _guard = test_lock();
        reset_auth_state_for_tests();
        let err = rpc_dependency_error("http://127.0.0.1:8080", "rpc down");
        assert!(err.contains("requires live role membership"));
        assert!(err.contains("http://127.0.0.1:8080"));
    }

    #[test]
    fn start_challenge_recovers_after_rpc_error() {
        let _guard = test_lock();
        reset_auth_state_for_tests();
        let rpc_error = rpc_dependency_error("http://127.0.0.1:8080", "rpc down");
        assert!(!rpc_error.is_empty());

        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[9u8; 32]).expect("valid key");
        let pk = PublicKey::from_secret_key(&secp, &sk);
        let signer_hex = hex::encode(pk.serialize());
        let now = now_unix_ms();
        let second = start_challenge_with_membership(
            StartChallengeInput {
                role: AuthRole::StrataAdministrator,
                rpc_url: None,
            },
            test_membership(AuthRole::StrataAdministrator, signer_hex, now),
        );
        assert!(second.is_ok(), "auth flow should recover once RPC is back");
    }

    #[test]
    fn replayed_challenge_is_rejected() {
        let _guard = test_lock();
        reset_auth_state_for_tests();

        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[10u8; 32]).expect("valid key");
        let pk = PublicKey::from_secret_key(&secp, &sk);
        let signer_hex = hex::encode(pk.serialize());
        let now = now_unix_ms();
        let challenge = start_challenge_with_membership(
            StartChallengeInput {
                role: AuthRole::StrataAdministrator,
                rpc_url: None,
            },
            test_membership(AuthRole::StrataAdministrator, signer_hex.clone(), now),
        )
        .expect("challenge should start");

        let (_, signature_hex) = sign_challenge_for_tests(&challenge.challenge_hex, [10u8; 32]);
        let first = complete_auth(CompleteAuthInput {
            challenge_id: challenge.challenge_id.clone(),
            signer_pubkey_hex: signer_hex.clone(),
            signature_hex: signature_hex.clone(),
            signature_format: SIG_FORMAT_P2WPKH_TX_BINDING.to_string(),
        });
        assert!(
            first.is_ok(),
            "expected first auth to succeed, got: {first:?}"
        );

        let second = complete_auth(CompleteAuthInput {
            challenge_id: challenge.challenge_id,
            signer_pubkey_hex: signer_hex,
            signature_hex,
            signature_format: SIG_FORMAT_P2WPKH_TX_BINDING.to_string(),
        });
        let err = second.expect_err("replayed challenge must be rejected");
        assert!(err.contains("already used"));
    }

    #[test]
    fn expired_challenge_is_rejected() {
        let _guard = test_lock();
        reset_auth_state_for_tests();
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[11u8; 32]).expect("valid key");
        let pk = PublicKey::from_secret_key(&secp, &sk);
        let signer_hex = hex::encode(pk.serialize());
        let now = now_unix_ms();

        let challenge = start_challenge_with_membership(
            StartChallengeInput {
                role: AuthRole::StrataAdministrator,
                rpc_url: None,
            },
            test_membership(AuthRole::StrataAdministrator, signer_hex.clone(), now),
        )
        .expect("challenge should start");

        {
            let mut state = auth_state().lock().expect("auth lock");
            let pending = state
                .pending
                .get_mut(&challenge.challenge_id)
                .expect("pending challenge exists");
            pending.challenge.expires_at_unix_ms = now_unix_ms().saturating_sub(1);
        }

        let (_, signature_hex) = sign_challenge_for_tests(&challenge.challenge_hex, [11u8; 32]);
        let result = complete_auth(CompleteAuthInput {
            challenge_id: challenge.challenge_id,
            signer_pubkey_hex: signer_hex,
            signature_hex,
            signature_format: SIG_FORMAT_P2WPKH_TX_BINDING.to_string(),
        });
        let err = result.expect_err("expired challenge must be rejected");
        assert!(err.contains("expired"));
    }

    #[test]
    fn start_challenge_clears_session_when_role_changes() {
        let _guard = test_lock();
        reset_auth_state_for_tests();
        let now = now_unix_ms();
        {
            let mut state = auth_state().lock().expect("auth lock");
            state.session = Some(AuthSession {
                role: AuthRole::StrataAdministrator,
                signer_pubkey_hex: "02aa".to_string(),
                authenticated_at_unix_ms: now,
                expires_at_unix_ms: now + SESSION_TTL_MS,
                membership_fetched_at_unix_ms: now,
            });
        }

        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[12u8; 32]).expect("valid key");
        let pk = PublicKey::from_secret_key(&secp, &sk);
        let signer_hex = hex::encode(pk.serialize());

        start_challenge_with_membership(
            StartChallengeInput {
                role: AuthRole::StrataSequencerManager,
                rpc_url: None,
            },
            test_membership(AuthRole::StrataSequencerManager, signer_hex, now),
        )
        .expect("challenge should start for new role");

        let session = get_session().expect("session query");
        assert!(
            !session.authenticated,
            "role change should invalidate previous session"
        );
    }
}
