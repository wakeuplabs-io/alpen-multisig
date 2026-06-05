/// How many days a pending proposal stays valid before it expires.
/// Must match the orchestrator's PROPOSAL_EXPIRY_DAYS setting.
pub const PROPOSAL_EXPIRY_DAYS: u64 = 7;

pub const LOCAL_STRATA_RPC_URL: &str = "http://127.0.0.1:8080";
pub const LOCAL_BTC_RPC_URL: &str = "http://127.0.0.1:18443";
pub const LOCAL_BTC_RPC_USER: &str = "user";
pub const LOCAL_BTC_RPC_PASS: &str = "password";
pub const TRUSTED_STRATA_RPC_URL: &str = "https://rpc.stratabtc.org";
pub const TRUSTED_BTC_RPC_URL: &str = "https://btc-rpc.stratabtc.org";
