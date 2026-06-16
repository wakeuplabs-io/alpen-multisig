//! Application layer — business logic for backend communication.
//!
//! `proposals` is the entry point for domain operations.
//! `orchestrator_client` defines the HTTP client contract (trait + DTOs);
//! the implementation lives in `crate::infrastructure`.

pub mod authentication;
pub mod commit_funding;
pub mod fee_estimation;
pub mod orchestrator_auth;
pub mod orchestrator_client;
pub mod orchestrator_url;
pub mod pending_reveals;
pub mod proposals;
pub mod psbt_signer;
pub mod tx_broadcaster;
pub mod wallet_send;
pub mod wallet_service;
pub mod wallet_session;
pub mod wallet_transactions;
