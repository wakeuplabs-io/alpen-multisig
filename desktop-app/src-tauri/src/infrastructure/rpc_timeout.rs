//! Shared reqwest client factory with a 30-second timeout for all external RPC calls (P-027).

use std::sync::OnceLock;
use std::time::Duration;

pub const RPC_TIMEOUT: Duration = Duration::from_secs(30);

static RPC_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Return a shared reqwest client pre-configured with the standard RPC timeout.
///
/// `reqwest::Client` is internally an `Arc` and is meant to be built once and cloned for
/// reuse, so its connection pool (and keep-alive) actually get used across calls.
pub fn rpc_client() -> reqwest::Client {
    RPC_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(RPC_TIMEOUT)
                .build()
                .expect("failed to build rpc client")
        })
        .clone()
}
