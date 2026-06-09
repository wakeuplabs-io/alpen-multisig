use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Response metadata for each error variant.
struct ErrorMeta {
    status: StatusCode,
    error_code: &'static str,
}

impl AppError {
    fn meta(&self) -> ErrorMeta {
        match self {
            AppError::Unauthorized => ErrorMeta {
                status: StatusCode::UNAUTHORIZED,
                error_code: "unauthorized",
            },
            AppError::NotFound => ErrorMeta {
                status: StatusCode::NOT_FOUND,
                error_code: "not_found",
            },
            AppError::BadRequest(_) => ErrorMeta {
                status: StatusCode::BAD_REQUEST,
                error_code: "bad_request",
            },
            AppError::Conflict(_) => ErrorMeta {
                status: StatusCode::CONFLICT,
                error_code: "conflict",
            },
            AppError::Internal(_) => ErrorMeta {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                error_code: "internal_error",
            },
        }
    }
}

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
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let meta = self.meta();
        let message = match &self {
            AppError::Internal(e) => {
                tracing::error!("internal error: {e}");
                "internal error".to_string()
            }
            other => other.to_string(),
        };

        (
            meta.status,
            Json(json!({ "error": message, "errorCode": meta.error_code })),
        )
            .into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
