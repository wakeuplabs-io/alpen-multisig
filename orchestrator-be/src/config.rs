use anyhow::Context;
use bitcoin::Network;

/// Publicly documented regtest operator key — must not be used without explicit opt-in.
pub const WELL_KNOWN_TEST_OPERATOR_KEY_HEX: &str =
    "0000000000000000000000000000000000000000000000000000000000000001";

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
    pub bitcoin_network: Network,
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
            operator_secret_key_hex: load_operator_secret_key_hex()?,
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
            bitcoin_network: parse_bitcoin_network(
                &std::env::var("BITCOIN_NETWORK")
                    .context("BITCOIN_NETWORK must be set (bitcoin|testnet|signet|regtest)")?,
            )?,
        })
    }
}

pub(crate) fn parse_bitcoin_network(name: &str) -> anyhow::Result<Network> {
    match name.trim() {
        "bitcoin" | "mainnet" => Ok(Network::Bitcoin),
        "testnet" => Ok(Network::Testnet),
        "signet" => Ok(Network::Signet),
        "regtest" => Ok(Network::Regtest),
        other => anyhow::bail!(
            "unknown BITCOIN_NETWORK '{other}'; expected bitcoin|testnet|signet|regtest"
        ),
    }
}

fn load_operator_secret_key_hex() -> anyhow::Result<String> {
    let key = match std::env::var("OPERATOR_SECRET_KEY_HEX") {
        Ok(k) => k,
        Err(_) if cfg!(test) => WELL_KNOWN_TEST_OPERATOR_KEY_HEX.to_string(),
        Err(_) => anyhow::bail!("OPERATOR_SECRET_KEY_HEX must be set"),
    };

    let allow_test = std::env::var("ORCHESTRATOR_ALLOW_TEST_OPERATOR_KEY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !allow_test {
        validate_operator_secret_key_hex(&key)?;
    }

    Ok(key)
}

/// Rejects the well-known test operator key unless dev opt-in is enabled.
pub(crate) fn validate_operator_secret_key_hex(hex: &str) -> anyhow::Result<()> {
    if hex.trim().eq_ignore_ascii_case(WELL_KNOWN_TEST_OPERATOR_KEY_HEX) {
        anyhow::bail!(
            "OPERATOR_SECRET_KEY_HEX must not be the well-known test key; \
             set ORCHESTRATOR_ALLOW_TEST_OPERATOR_KEY=1 only for local regtest"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_well_known_test_operator_key() {
        let err = validate_operator_secret_key_hex(WELL_KNOWN_TEST_OPERATOR_KEY_HEX).unwrap_err();
        assert!(err.to_string().contains("well-known test key"));
    }

    #[test]
    fn accepts_non_test_operator_key() {
        validate_operator_secret_key_hex(
            "0000000000000000000000000000000000000000000000000000000000000002",
        )
        .unwrap();
    }

    #[test]
    fn parse_bitcoin_network_requires_explicit_name() {
        assert!(parse_bitcoin_network("regtest").is_ok());
        assert!(parse_bitcoin_network("").is_err());
        assert!(parse_bitcoin_network("mainnet2").is_err());
    }
}
