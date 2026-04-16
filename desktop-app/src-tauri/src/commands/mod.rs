//! Thin Tauri command wrappers. Each command extracts State, delegates to
//! the application layer, and maps errors to String for the IPC boundary.

pub mod auth;
pub mod hw_wallet;
pub mod proposals;
