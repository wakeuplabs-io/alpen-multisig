use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::{
    domain::auth::{AuthSession, PendingAuthChallenge},
    domain::authority::Authority,
    error::{AppError, Result},
    infrastructure::{asm_role_membership, auth_crypto},
    state::AppState,
};

const SIG_FORMAT_P2WPKH_TX_BINDING: &str = "p2wpkh-tx-binding";
const SIG_FORMAT_BITCOIN_MESSAGE: &str = "bitcoin-message";
const SIG_FORMAT_RAW_ECDSA: &str = "raw-ecdsa";

#[derive(Debug, Deserialize)]
pub struct StartAuthChallengeRequest {
    pub authority: Authority,
}

#[derive(Debug, Serialize)]
pub struct AuthChallengeResponse {
    pub challenge_id: String,
    pub challenge_hex: String,
    pub issued_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct VerifyAuthRequest {
    pub challenge_id: String,
    pub signer_pubkey: String,
    pub signature_hex: String,
    pub signature_format: String,
}

#[derive(Debug, Serialize)]
pub struct AuthSessionResponse {
    pub token: String,
    pub authority: Authority,
    pub signer_pubkey: String,
    pub expires_at_unix_ms: u64,
}

pub async fn auth_challenge(
    State(state): State<AppState>,
    Json(body): Json<StartAuthChallengeRequest>,
) -> Result<Json<AuthChallengeResponse>> {
    let now = now_unix_ms();
    let expires_at_unix_ms = now + state.auth_challenge_ttl_ms;
    let challenge_id = auth_crypto::random_hex(16);
    let nonce_hex = auth_crypto::random_hex(32);
    let challenge_hex = hex::encode(auth_crypto::create_challenge_digest(
        authority_wire(body.authority),
        &nonce_hex,
        now,
        expires_at_unix_ms,
        &challenge_id,
    ));

    let challenge = PendingAuthChallenge {
        authority: body.authority,
        challenge_hex: challenge_hex.clone(),
        expires_at_unix_ms,
        consumed: false,
    };
    state
        .challenges
        .write()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("challenge lock poisoned")))?
        .insert(challenge_id.clone(), challenge);

    Ok(Json(AuthChallengeResponse {
        challenge_id,
        challenge_hex,
        issued_at_unix_ms: now,
        expires_at_unix_ms,
    }))
}

pub async fn auth_verify(
    State(state): State<AppState>,
    Json(body): Json<VerifyAuthRequest>,
) -> Result<(StatusCode, Json<AuthSessionResponse>)> {
    if body.signature_format != SIG_FORMAT_P2WPKH_TX_BINDING
        && body.signature_format != SIG_FORMAT_BITCOIN_MESSAGE
        && body.signature_format != SIG_FORMAT_RAW_ECDSA
    {
        return Err(AppError::Unauthorized);
    }

    let now = now_unix_ms();
    let authority = {
        let mut challenges = state
            .challenges
            .write()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("challenge lock poisoned")))?;
        let challenge = challenges
            .get_mut(&body.challenge_id)
            .ok_or(AppError::Unauthorized)?;
        if challenge.consumed || now > challenge.expires_at_unix_ms {
            return Err(AppError::Unauthorized);
        }

        if body.signature_format == SIG_FORMAT_BITCOIN_MESSAGE {
            auth_crypto::verify_bitcoin_message_signature(
                &challenge.challenge_hex,
                &body.signer_pubkey,
                &body.signature_hex,
            )
            .map_err(|_| AppError::Unauthorized)?;
        } else {
            // raw-ecdsa and p2wpkh-tx-binding both use plain ECDSA on the raw challenge digest.
            auth_crypto::verify_signature(
                &challenge.challenge_hex,
                &body.signer_pubkey,
                &body.signature_hex,
            )
            .map_err(|_| AppError::Unauthorized)?;
        }
        challenge.consumed = true;
        challenge.authority
    };

    let is_member = asm_role_membership::is_signer_member_for_authority(
        &state.asm_rpc_url,
        authority,
        &body.signer_pubkey,
    )
    .await?;
    if !is_member {
        return Err(AppError::Unauthorized);
    }

    let token = auth_crypto::random_hex(32);
    let expires_at_unix_ms = now + state.auth_session_ttl_ms;
    let session = AuthSession {
        authority,
        signer_pubkey: body.signer_pubkey.clone(),
        expires_at_unix_ms,
    };
    state
        .sessions
        .write()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("session lock poisoned")))?
        .insert(token.clone(), session.clone());

    Ok((
        StatusCode::CREATED,
        Json(AuthSessionResponse {
            token,
            authority: session.authority,
            signer_pubkey: session.signer_pubkey,
            expires_at_unix_ms,
        }),
    ))
}

pub async fn auth_logout(
    State(state): State<AppState>,
    token: super::auth_session::AuthenticatedSession,
) -> Result<StatusCode> {
    state
        .sessions
        .write()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("session lock poisoned")))?
        .remove(&token.token);
    Ok(StatusCode::NO_CONTENT)
}

fn authority_wire(authority: Authority) -> &'static str {
    match authority {
        Authority::AlpenAdmin => "alpen_admin",
        Authority::StrataAdmin => "strata_admin",
        Authority::SequencerManager => "sequencer_manager",
        Authority::SecurityCouncil => "security_council",
        Authority::PayoutAdmin => "payout_admin",
    }
}

fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as u64
}
