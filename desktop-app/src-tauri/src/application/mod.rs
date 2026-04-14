//! Application layer — business logic for backend communication.
//!
//! `proposals` is the entry point for domain operations.
//! `orchestrator_client` defines the HTTP client contract (trait + DTOs);
//! the implementation lives in `crate::infrastructure`.

pub mod auth;
pub mod orchestrator_client;
pub mod proposals;
