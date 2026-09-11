//! Shared reqwest client factory with a 30-second timeout for all external RPC calls (P-027).

use std::time::Duration;

pub const RPC_TIMEOUT: Duration = Duration::from_secs(30);

/// Build a reqwest client pre-configured with the standard RPC timeout.
pub fn rpc_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(RPC_TIMEOUT)
        .build()
        .expect("failed to build rpc client")
}
