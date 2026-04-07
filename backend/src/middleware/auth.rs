use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use crate::{domain::session::Session, error::AppError, state::AppState};

/// Extractor that validates the session token from the Authorization header
/// and injects the authenticated session into handler arguments.
pub struct AuthenticatedSession(pub Session);

#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedSession {
	type Rejection = AppError;

	async fn from_request_parts(
		_parts: &mut Parts,
		_state: &AppState,
	) -> Result<Self, Self::Rejection> {
		todo!("extract Bearer token, look up session, verify not expired")
	}
}
