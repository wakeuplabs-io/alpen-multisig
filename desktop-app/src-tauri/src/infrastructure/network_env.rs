//! Canonical Bitcoin network resolution for the desktop process.
//!
//! The active network is a process-wide flag read from `BITCOIN_NETWORK`
//! (defaulting to regtest), **not** a per-connection setting. [`NodeConfig`]
//! holds only RPC endpoints/credentials — it intentionally does not carry the
//! network. Keeping this in one place avoids each command re-reading and
//! re-parsing the env var with subtly different fallbacks.
//!
//! [`NodeConfig`]: crate::infrastructure::node_config_store::NodeConfig

use bdk_wallet::bitcoin::Network;

#[derive(Debug, thiserror::Error)]
#[error("invalid Bitcoin network '{0}'; expected bitcoin/testnet/signet/regtest")]
pub struct InvalidNetwork(pub String);

/// Parses a canonical network token. Mirrors the values written by the env
/// loader; unknown tokens are rejected rather than silently coerced.
pub fn parse_network(network: &str) -> Result<Network, InvalidNetwork> {
    match network {
        "bitcoin" => Ok(Network::Bitcoin),
        "testnet" => Ok(Network::Testnet),
        "signet" => Ok(Network::Signet),
        "regtest" => Ok(Network::Regtest),
        other => Err(InvalidNetwork(other.to_string())),
    }
}

/// Resolves the active network from `BITCOIN_NETWORK`, defaulting to regtest
/// when the variable is absent. A present-but-malformed value is an error.
pub fn network_from_env() -> Result<Network, InvalidNetwork> {
    let network = std::env::var("BITCOIN_NETWORK").unwrap_or_else(|_| "regtest".to_string());
    parse_network(&network)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::broadcast_env::ENV_TEST_LOCK;

    #[test]
    fn parses_canonical_tokens() {
        assert_eq!(parse_network("bitcoin").unwrap(), Network::Bitcoin);
        assert_eq!(parse_network("testnet").unwrap(), Network::Testnet);
        assert_eq!(parse_network("signet").unwrap(), Network::Signet);
        assert_eq!(parse_network("regtest").unwrap(), Network::Regtest);
    }

    #[test]
    fn rejects_unknown_token() {
        let err = parse_network("mainnet").unwrap_err();
        assert_eq!(err.0, "mainnet");
    }

    #[test]
    fn defaults_to_regtest_when_env_absent() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        std::env::remove_var("BITCOIN_NETWORK");
        assert_eq!(network_from_env().unwrap(), Network::Regtest);
    }

    #[test]
    fn reads_network_from_env() {
        let _guard = ENV_TEST_LOCK.lock().unwrap();
        std::env::set_var("BITCOIN_NETWORK", "signet");
        assert_eq!(network_from_env().unwrap(), Network::Signet);
        std::env::remove_var("BITCOIN_NETWORK");
    }
}
