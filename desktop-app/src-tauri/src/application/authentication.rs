use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::domain::auth::{
    AuthChallenge, AuthRole, AuthSession, MembershipCache, PendingChallenge,
};
use crate::infrastructure::{asm_status_rpc, challenge_verifier};

const CHALLENGE_TTL_MS: u64 = 120_000;
const SESSION_TTL_MS: u64 = 600_000;
const MEMBERSHIP_MAX_AGE_MS: u64 = 300_000;
const SIG_FORMAT_P2WPKH_TX_BINDING: &str = "p2wpkh-tx-binding";
const SIG_FORMAT_BITCOIN_MESSAGE: &str = "bitcoin-message";

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

    let (role_to_keys, fetched_at_unix_ms) =
        asm_status_rpc::fetch_role_membership(&rpc_url).await?;
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

pub fn complete_auth(input: CompleteAuthInput) -> Result<AuthSession, String> {
    let signature_format = input.signature_format.as_str();
    if signature_format != SIG_FORMAT_P2WPKH_TX_BINDING
        && signature_format != SIG_FORMAT_BITCOIN_MESSAGE
    {
        return Err(format!(
            "unsupported signature format `{}`; expected one of: `{}`, `{}`",
            input.signature_format, SIG_FORMAT_P2WPKH_TX_BINDING, SIG_FORMAT_BITCOIN_MESSAGE
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
    if now.saturating_sub(membership.fetched_at_unix_ms) > MEMBERSHIP_MAX_AGE_MS {
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
    match role {
        AuthRole::StrataAdministrator => "strata_administrator",
        AuthRole::StrataSequencerManager => "strata_sequencer_manager",
    }
}

fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as u64
}
