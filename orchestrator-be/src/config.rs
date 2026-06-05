use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub auth_challenge_ttl_ms: u64,
    pub auth_session_ttl_ms: u64,
    pub strata_admin_state_rpc_url: String,
    pub database_url: Option<String>,
    /// Bitcoin RPC for `/ready` health checks only (broadcast runs in the desktop app).
    pub bitcoin_rpc_url: String,
    pub bitcoin_rpc_user: String,
    pub bitcoin_rpc_pass: String,
    /// How many days a pending proposal stays valid before it expires. Env: PROPOSAL_EXPIRY_DAYS.
    pub proposal_expiry_days: u64,
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
            database_url: {
                let url = std::env::var("DATABASE_URL")
                    .ok()
                    .map(|s| s.trim().to_string());
                let profile = std::env::var("ORCHESTRATOR_PROFILE").unwrap_or_default();
                if profile.eq_ignore_ascii_case("production") && url.is_none() {
                    anyhow::bail!("DATABASE_URL is required when ORCHESTRATOR_PROFILE=production");
                }
                url
            },
            bitcoin_rpc_url: std::env::var("BITCOIN_RPC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:18443".to_string()),
            bitcoin_rpc_user: std::env::var("BITCOIN_RPC_USER")
                .unwrap_or_else(|_| "rpcuser".to_string()),
            bitcoin_rpc_pass: std::env::var("BITCOIN_RPC_PASS")
                .unwrap_or_else(|_| "rpcpass".to_string()),
            proposal_expiry_days: std::env::var("PROPOSAL_EXPIRY_DAYS")
                .unwrap_or_else(|_| "7".to_string())
                .parse()
                .context("PROPOSAL_EXPIRY_DAYS must be a valid u64")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_profile_requires_database_url() {
        std::env::set_var("ORCHESTRATOR_PROFILE", "production");
        std::env::remove_var("DATABASE_URL");
        let err = Config::from_env().unwrap_err();
        assert!(err.to_string().contains("DATABASE_URL"));
        std::env::remove_var("ORCHESTRATOR_PROFILE");
    }
}
