use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};

use crate::{domain::authority::Authority, error::AppError, state::AppState};

#[derive(Debug, Clone)]
pub struct AuthenticatedSession {
    pub token: String,
    pub authority: Authority,
    pub signer_pubkey: String,
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthenticatedSession {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> std::result::Result<Self, Self::Rejection> {
        let auth = parts
            .headers
            .get(AUTHORIZATION)
            .ok_or(AppError::Unauthorized)?
            .to_str()
            .map_err(|_| AppError::Unauthorized)?;
        let token = auth
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?
            .to_string();

        let now = now_unix_ms();
        let sessions = state
            .sessions
            .read()
            .map_err(|_| AppError::Internal(anyhow::anyhow!("session lock poisoned")))?;
        let session = sessions.get(&token).ok_or(AppError::Unauthorized)?;
        if now > session.expires_at_unix_ms {
            return Err(AppError::Unauthorized);
        }

        Ok(Self {
            token,
            authority: session.authority,
            signer_pubkey: session.signer_pubkey.clone(),
        })
    }
}

fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_millis() as u64
}
