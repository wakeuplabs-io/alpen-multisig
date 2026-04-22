use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    /// Required for auth endpoint — can be None in dev/test if auth is never called.
    pub strata_rpc_url: Option<String>,
    pub strata_rpc_method: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            server_host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("SERVER_PORT must be a valid port number")?,
            strata_rpc_url: std::env::var("STRATA_ADMIN_STATE_RPC_URL").ok(),
            strata_rpc_method: std::env::var("STRATA_ADMIN_STATE_RPC_METHOD")
                .unwrap_or_else(|_| "strata_getAdminState".to_string()),
        })
    }
}
