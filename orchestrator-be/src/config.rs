use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub auth_challenge_ttl_ms: u64,
    pub auth_session_ttl_ms: u64,
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
        })
    }
}
