use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,

    #[error("not found")]
    NotFound,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("internal error")]
    Internal(#[from] anyhow::Error),

    /// Hardware wallet device physically disconnected or absent.
    /// Used by hardware device adapters (Ledger/Trezor) when device is not present.
    #[allow(dead_code)]
    #[error("hardware wallet device disconnected")]
    HwDisconnected,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message, error_code) = match &self {
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized".to_string(),
                "unauthorized",
            ),
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string(), "not_found"),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone(), "bad_request"),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone(), "conflict"),
            AppError::HwDisconnected => (
                StatusCode::PRECONDITION_FAILED,
                self.to_string(),
                "hw_disconnected",
            ),
            AppError::Internal(e) => {
                tracing::error!("internal error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                    "internal_error",
                )
            }
        };

        (
            status,
            Json(json!({ "error": message, "errorCode": error_code })),
        )
            .into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
