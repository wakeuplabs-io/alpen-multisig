//! Application layer — business logic for backend communication.
//!
//! - `auth` handles the challenge/session lifecycle.
//! - `proposals` orchestrates proposal operations via the `OrchestratorClient` trait.
//! - `traits` defines the `OrchestratorClient` trait and `OrchestratorError`.

pub mod auth;
pub mod proposals;
pub mod traits;
