//! Shared reqwest client for external RPC calls.
//!
//! `reqwest::Client` is internally an `Arc` and is meant to be built once and cloned for
//! reuse, so its connection pool (and keep-alive) actually get used across calls instead of
//! a fresh pool per request.
//!
//! Deliberately no `.timeout(...)` here: `reqwest::Client::new()` (what this replaces) has
//! never had one, and adding one now would be a behavior change beyond this cleanup. Timeouts
//! for these calls are already enforced by `rpc_timeout::with_rpc_timeout` at the call site.

use std::sync::OnceLock;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Return a shared reqwest client with default configuration (no timeout).
pub(crate) fn shared() -> reqwest::Client {
    HTTP_CLIENT.get_or_init(reqwest::Client::new).clone()
}
