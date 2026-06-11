/// How many days a pending proposal stays valid before it expires.
/// Must match the orchestrator's PROPOSAL_EXPIRY_DAYS setting.
pub const PROPOSAL_EXPIRY_DAYS: u64 = 1;

pub const LOCAL_STRATA_RPC_URL: &str = "http://127.0.0.1:8080";
pub const LOCAL_BTC_RPC_URL: &str = "http://127.0.0.1:18443";
pub const LOCAL_BTC_RPC_USER: &str = "user";
pub const LOCAL_BTC_RPC_PASS: &str = "password";
pub const TRUSTED_STRATA_RPC_URL: &str = "https://rpc.stratabtc.org";
pub const TRUSTED_BTC_RPC_URL: &str = "https://btc-rpc.stratabtc.org";

/// Local electrs endpoint from `scripts/local-stack.sh` (R2.1 indexer infra).
pub const LOCAL_ELECTRUM_URL: &str = "tcp://127.0.0.1:60401";

/// Trusted Electrum preset following the stratabtc.org endpoint naming pattern. The official
/// public indexer endpoint is published/hardened in Phase 10; until then, Trusted-mode wallet
/// sync surfaces `ElectrumUnreachable` if this host is not live.
pub const TRUSTED_ELECTRUM_URL: &str = "ssl://electrum.stratabtc.org:50002";
