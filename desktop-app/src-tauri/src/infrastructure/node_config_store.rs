use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const CONFIG_FILE: &str = "node-config.json";

const LOCAL_STRATA_RPC_URL: &str = "http://127.0.0.1:8080";
const LOCAL_BTC_RPC_URL: &str = "http://127.0.0.1:18443";
const TRUSTED_STRATA_RPC_URL: &str = "https://rpc.stratabtc.org";
const TRUSTED_BTC_RPC_URL: &str = "https://btc-rpc.stratabtc.org";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionMode {
    #[default]
    Local,
    Trusted,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeConfig {
    #[serde(default)]
    pub mode: ConnectionMode,
    pub custom_strata_rpc_url: Option<String>,
    pub custom_btc_rpc_url: Option<String>,
    pub custom_btc_rpc_user: Option<String>,
    pub custom_btc_rpc_pass: Option<String>,
}

impl NodeConfig {
    pub fn strata_rpc_url(&self) -> &str {
        match &self.mode {
            ConnectionMode::Local => LOCAL_STRATA_RPC_URL,
            ConnectionMode::Trusted => TRUSTED_STRATA_RPC_URL,
            ConnectionMode::Custom => self
                .custom_strata_rpc_url
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(LOCAL_STRATA_RPC_URL),
        }
    }

    pub fn btc_rpc_url(&self) -> &str {
        match &self.mode {
            ConnectionMode::Local => LOCAL_BTC_RPC_URL,
            ConnectionMode::Trusted => TRUSTED_BTC_RPC_URL,
            ConnectionMode::Custom => self
                .custom_btc_rpc_url
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(LOCAL_BTC_RPC_URL),
        }
    }

    pub fn btc_rpc_user(&self) -> &str {
        self.custom_btc_rpc_user.as_deref().unwrap_or("")
    }

    pub fn btc_rpc_pass(&self) -> &str {
        self.custom_btc_rpc_pass.as_deref().unwrap_or("")
    }
}

pub struct NodeConfigState(pub Arc<RwLock<NodeConfig>>);

pub fn load_node_config(app: &AppHandle) -> NodeConfig {
    let config_dir = match app.path().app_config_dir() {
        Ok(dir) => dir,
        Err(_) => return NodeConfig::default(),
    };
    let path = config_dir.join(CONFIG_FILE);
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return NodeConfig::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

pub fn save_node_config(app: &AppHandle, config: &NodeConfig) -> Result<(), String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("failed to resolve app config dir: {e}"))?;
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("failed to create config dir: {e}"))?;
    let path = config_dir.join(CONFIG_FILE);
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize config: {e}"))?;
    std::fs::write(path, json).map_err(|e| format!("failed to write config: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_mode_returns_localhost_urls() {
        let cfg = NodeConfig::default();
        assert_eq!(cfg.strata_rpc_url(), LOCAL_STRATA_RPC_URL);
        assert_eq!(cfg.btc_rpc_url(), LOCAL_BTC_RPC_URL);
    }

    #[test]
    fn trusted_mode_returns_stratabtc_urls() {
        let cfg = NodeConfig {
            mode: ConnectionMode::Trusted,
            ..Default::default()
        };
        assert_eq!(cfg.strata_rpc_url(), TRUSTED_STRATA_RPC_URL);
        assert_eq!(cfg.btc_rpc_url(), TRUSTED_BTC_RPC_URL);
    }

    #[test]
    fn custom_mode_returns_custom_urls() {
        let cfg = NodeConfig {
            mode: ConnectionMode::Custom,
            custom_strata_rpc_url: Some("http://my-strata:9090".to_string()),
            custom_btc_rpc_url: Some("http://my-btc:8080".to_string()),
            ..Default::default()
        };
        assert_eq!(cfg.strata_rpc_url(), "http://my-strata:9090");
        assert_eq!(cfg.btc_rpc_url(), "http://my-btc:8080");
    }

    #[test]
    fn custom_mode_falls_back_to_local_when_urls_empty() {
        let cfg = NodeConfig {
            mode: ConnectionMode::Custom,
            ..Default::default()
        };
        assert_eq!(cfg.strata_rpc_url(), LOCAL_STRATA_RPC_URL);
        assert_eq!(cfg.btc_rpc_url(), LOCAL_BTC_RPC_URL);
    }
}
