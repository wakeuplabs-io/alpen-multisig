use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub auth_challenge_ttl_ms: u64,
    pub auth_session_ttl_ms: u64,
    pub strata_admin_state_rpc_url: String,
    pub database_url: String,
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
                .context("DATABASE_URL is required")?
                .trim()
                .to_string(),
        })
    }
}
