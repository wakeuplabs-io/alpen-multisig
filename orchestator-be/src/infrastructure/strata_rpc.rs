use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::authority::Authority;

#[derive(Debug, Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: [Value; 0],
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

const KEY_PATHS: [&str; 4] = [
    "/threshold_config/keys",
    "/strata_administrator/keys",
    "/authorities/strata_administrator/config/keys",
    "/authorities/StrataAdministrator/config/keys",
];

pub async fn fetch_signer_set(
    rpc_url: &str,
    rpc_method: &str,
    _authority: Authority,
) -> anyhow::Result<Vec<String>> {
    let request = JsonRpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: rpc_method,
        params: [],
    };

    let response: JsonRpcResponse = reqwest::Client::new()
        .post(rpc_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("failed to call Strata admin RPC: {e}"))?
        .error_for_status()
        .map_err(|e| anyhow::anyhow!("Strata admin RPC returned non-success status: {e}"))?
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("failed to decode Strata admin RPC response: {e}"))?;

    if let Some(error) = response.error {
        return Err(anyhow::anyhow!(
            "Strata admin RPC returned error {}: {}",
            error.code,
            error.message
        ));
    }

    let result = response
        .result
        .ok_or_else(|| anyhow::anyhow!("Strata admin RPC response missing result"))?;

    extract_keys(&result)
}

fn extract_keys(snapshot: &Value) -> anyhow::Result<Vec<String>> {
    for path in KEY_PATHS {
        if let Some(keys) = snapshot.pointer(path).and_then(Value::as_array) {
            return keys
                .iter()
                .map(parse_key_from_value)
                .collect::<anyhow::Result<Vec<_>>>();
        }
    }
    Err(anyhow::anyhow!(
        "failed to locate Strata administrator key set in RPC result"
    ))
}

fn parse_key_from_value(value: &Value) -> anyhow::Result<String> {
    let raw = if let Some(s) = value.as_str() {
        s.to_string()
    } else {
        value
            .as_object()
            .and_then(|o| {
                o.get("key")
                    .or_else(|| o.get("pubkey"))
                    .or_else(|| o.get("compressed_pubkey"))
            })
            .and_then(Value::as_str)
            .ok_or_else(|| {
                anyhow::anyhow!("admin key must be a hex string or object with key/pubkey field")
            })?
            .to_string()
    };
    normalize_pubkey_hex(&raw)
}

pub fn normalize_pubkey_hex(pubkey_hex: &str) -> anyhow::Result<String> {
    use secp256k1::PublicKey;
    let trimmed = pubkey_hex
        .trim()
        .strip_prefix("0x")
        .unwrap_or(pubkey_hex.trim());
    let bytes = hex::decode(trimmed).map_err(|e| anyhow::anyhow!("invalid pubkey hex: {e}"))?;
    let pk = PublicKey::from_slice(&bytes)
        .map_err(|e| anyhow::anyhow!("invalid secp256k1 public key: {e}"))?;
    Ok(hex::encode(pk.serialize()))
}
