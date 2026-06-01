use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use desktop_app::config::{LOCAL_BTC_RPC_URL, LOCAL_STRATA_RPC_URL};
use desktop_app::infrastructure::node_config_store::{
    self, ConnectionMode, NodeConfig, NodeConfigState,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeConfigDto {
    pub mode: String,
    pub custom_strata_rpc_url: Option<String>,
    pub custom_btc_rpc_url: Option<String>,
    pub custom_btc_rpc_user: Option<String>,
    pub custom_btc_rpc_pass: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalNodeStatusDto {
    pub strata_reachable: bool,
    pub btc_reachable: bool,
}

fn mode_to_str(mode: &ConnectionMode) -> &'static str {
    match mode {
        ConnectionMode::Local => "local",
        ConnectionMode::Trusted => "trusted",
        ConnectionMode::Custom => "custom",
    }
}

fn mode_from_str(s: &str) -> ConnectionMode {
    match s {
        "trusted" => ConnectionMode::Trusted,
        "custom" => ConnectionMode::Custom,
        _ => ConnectionMode::Local,
    }
}

#[tauri::command]
pub async fn get_node_config(state: State<'_, NodeConfigState>) -> Result<NodeConfigDto, String> {
    let cfg = state
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .clone();
    Ok(NodeConfigDto {
        mode: mode_to_str(&cfg.mode).to_string(),
        custom_strata_rpc_url: cfg.custom_strata_rpc_url,
        custom_btc_rpc_url: cfg.custom_btc_rpc_url,
        custom_btc_rpc_user: cfg.custom_btc_rpc_user,
        custom_btc_rpc_pass: cfg.custom_btc_rpc_pass,
    })
}

#[tauri::command]
pub async fn save_node_config(
    config: NodeConfigDto,
    state: State<'_, NodeConfigState>,
    app: AppHandle,
) -> Result<(), String> {
    let new_config = NodeConfig {
        mode: mode_from_str(&config.mode),
        custom_strata_rpc_url: config.custom_strata_rpc_url,
        custom_btc_rpc_url: config.custom_btc_rpc_url,
        custom_btc_rpc_user: config.custom_btc_rpc_user,
        custom_btc_rpc_pass: config.custom_btc_rpc_pass,
    };
    node_config_store::save_node_config(&app, &new_config)?;
    let mut guard = state.0.write().map_err(|e| format!("lock error: {e}"))?;
    *guard = new_config;
    Ok(())
}

#[tauri::command]
pub async fn check_local_node() -> Result<LocalNodeStatusDto, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| format!("failed to build http client: {e}"))?;

    let strata_reachable = client.get(LOCAL_STRATA_RPC_URL).send().await.is_ok();

    let btc_reachable = client.get(LOCAL_BTC_RPC_URL).send().await.is_ok();

    Ok(LocalNodeStatusDto {
        strata_reachable,
        btc_reachable,
    })
}
