//! Tauri IPC command boundary — thin wrappers that delegate to the application layer.

pub(crate) mod action_builder;
pub(crate) mod admin_wallet;
pub(crate) mod asm_state;
pub(crate) mod authentication;
pub(crate) mod hw_wallet;
pub(crate) mod invoke;
pub(crate) mod node_config;
pub(crate) mod orchestrator_auth;
pub(crate) mod proposals;
pub(crate) mod signing;
