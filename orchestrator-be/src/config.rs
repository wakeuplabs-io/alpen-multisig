use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub auth_challenge_ttl_ms: u64,
    pub auth_session_ttl_ms: u64,
    pub strata_admin_state_rpc_url: String,
    pub database_url: Option<String>,
    // Broadcast config
    pub bitcoin_rpc_url: String,
    pub bitcoin_rpc_user: String,
    pub bitcoin_rpc_pass: String,
    pub bitcoin_wallet_name: Option<String>,
    /// Hex-encoded 32-byte secret key used to sign reveal witness (operator key).
    pub operator_secret_key_hex: String,
    /// 4-byte magic bytes as hex string (e.g. "414c504e" for "ALPN"). Matches the ASM deployment.
    pub bitcoin_magic_bytes_hex: String,
    pub confirm_poll_interval_ms: u64,
    pub confirm_timeout_ms: u64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            server_host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("SERVER_PORT must be a valid port number")?,
            auth_challenge_ttl_ms: std::env::var("AUTH_CHALLENGE_TTL_MS")
                .unwrap_or_else(|_| "120000".to_string())
                .parse()
                .context("AUTH_CHALLENGE_TTL_MS must be a valid u64")?,
            auth_session_ttl_ms: std::env::var("AUTH_SESSION_TTL_MS")
                .unwrap_or_else(|_| "240000".to_string())
                .parse()
                .context("AUTH_SESSION_TTL_MS must be a valid u64")?,
            strata_admin_state_rpc_url: std::env::var("STRATA_ADMIN_STATE_RPC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
                .trim()
                .to_string(),
            database_url: std::env::var("DATABASE_URL")
                .ok()
                .map(|s| s.trim().to_string()),
            bitcoin_rpc_url: std::env::var("BITCOIN_RPC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:18443".to_string()),
            bitcoin_rpc_user: std::env::var("BITCOIN_RPC_USER")
                .unwrap_or_else(|_| "rpcuser".to_string()),
            bitcoin_rpc_pass: std::env::var("BITCOIN_RPC_PASS")
                .unwrap_or_else(|_| "rpcpass".to_string()),
            bitcoin_wallet_name: std::env::var("BITCOIN_WALLET_NAME")
                .ok()
                .filter(|s| !s.is_empty()),
            operator_secret_key_hex: std::env::var("OPERATOR_SECRET_KEY_HEX").unwrap_or_else(
                |_| {
                    // Deterministic test key (32 bytes, value = 1); override in production.
                    "0000000000000000000000000000000000000000000000000000000000000001".to_string()
                },
            ),
            // "ALPN" in hex = 414c504e; matches TEST_MAGIC_BYTES used in e2e tests.
            bitcoin_magic_bytes_hex: std::env::var("BITCOIN_MAGIC_BYTES_HEX")
                .unwrap_or_else(|_| "414c504e".to_string()),
            confirm_poll_interval_ms: std::env::var("CONFIRM_POLL_INTERVAL_MS")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .context("CONFIRM_POLL_INTERVAL_MS must be a valid u64")?,
            confirm_timeout_ms: std::env::var("CONFIRM_TIMEOUT_MS")
                .unwrap_or_else(|_| "600000".to_string())
                .parse()
                .context("CONFIRM_TIMEOUT_MS must be a valid u64")?,
        })
    }
}
