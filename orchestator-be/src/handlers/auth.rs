use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::application::auth::{self, AuthRequest};
use crate::domain::authority::Authority;
use crate::error::{AppError, Result};
use crate::infrastructure::strata_rpc;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct AuthRequestBody {
    pub authority: String,
    pub signer_pubkey: String,
    pub ephemeral_pubkey: String,
    pub nonce: String,
    pub expiry_secs: u64,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponseBody {
    pub session_token: String,
    pub expires_at: i64,
}

pub async fn authenticate(
    State(state): State<AppState>,
    Json(body): Json<AuthRequestBody>,
) -> Result<Json<AuthResponseBody>> {
    let authority = Authority::from_wire(&body.authority).map_err(AppError::BadRequest)?;

    let rpc_url = state.config.strata_rpc_url.clone().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "STRATA_ADMIN_STATE_RPC_URL not configured; cannot verify signer set"
        ))
    })?;
    let rpc_method = state.config.strata_rpc_method.clone();

    let resp = auth::authenticate(
        AuthRequest {
            authority,
            signer_pubkey: body.signer_pubkey,
            ephemeral_pubkey: body.ephemeral_pubkey,
            nonce: body.nonce,
            expiry_secs: body.expiry_secs,
            signature: body.signature,
        },
        &state.config,
        &state.sessions,
        &state.used_nonces,
        |auth| async move { strata_rpc::fetch_signer_set(&rpc_url, &rpc_method, auth).await },
    )
    .await?;

    Ok(Json(AuthResponseBody {
        session_token: resp.session_token,
        expires_at: resp.expires_at,
    }))
}
