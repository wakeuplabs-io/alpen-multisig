use axum::{Json, Router, routing::{get, post}};
use serde_json::{json, Value};
use crate::state::AppState;

pub mod auth;
pub mod proposals;
pub mod signatures;

async fn health() -> Json<Value> {
	Json(json!({ "status": "ok" }))
}

pub fn router(state: AppState) -> Router {
	Router::new()
		.route("/health", get(health))
		// Auth
		.route("/auth/challenge", get(auth::get_challenge))
		.route("/auth/session", post(auth::create_session))
		.route("/auth/session", axum::routing::delete(auth::delete_session))
		// Proposals
		.route("/proposals", get(proposals::list_proposals))
		.route("/proposals", post(proposals::create_proposal))
		.route("/proposals/:action_id", get(proposals::get_proposal))
		// Signatures
		.route("/proposals/:action_id/signatures", post(signatures::submit_signature))
		.route("/proposals/:action_id/signatures", get(signatures::list_signatures))
		.with_state(state)
}
