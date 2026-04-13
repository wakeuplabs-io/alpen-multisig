//! Desktop application library — exposes the application and domain layers for external consumers.
//!
//! - `domain` holds pure client-side types (proposals, sessions, authority).
//! - `application` holds business logic (proposal operations, auth lifecycle).
//! - `infrastructure` holds concrete implementations (HTTP client, hardware wallets).
//! - `signing` provides cryptographic signing utilities.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod signing;
