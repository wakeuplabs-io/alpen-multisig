use bitcoin::secp256k1::PublicKey;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use strata_crypto::keys::compressed::CompressedPublicKey;

#[derive(Debug, Serialize)]
pub struct AuthorityEligibility {
    pub authority: String,
    pub eligible: bool,
}

const DEFAULT_RPC_METHOD: &str = "strata_getAdminState";

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

#[tauri::command]
pub(crate) async fn check_strata_admin_signer(
    signer_pubkey_hex: String,
) -> Result<AuthorityEligibility, String> {
    let rpc_url = std::env::var("STRATA_ADMIN_STATE_RPC_URL").map_err(|_| {
        "STRATA_ADMIN_STATE_RPC_URL is required to check canonical signer membership".to_string()
    })?;
    let rpc_method = std::env::var("STRATA_ADMIN_STATE_RPC_METHOD")
        .unwrap_or_else(|_| DEFAULT_RPC_METHOD.to_string());

    let signer = normalize_signer_pubkey(&signer_pubkey_hex)?;
    let url = reqwest::Url::parse(&rpc_url)
        .map_err(|e| format!("invalid STRATA_ADMIN_STATE_RPC_URL: {e}"))?;
    let request = JsonRpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: &rpc_method,
        params: [],
    };

    let response: JsonRpcResponse = reqwest::Client::new()
        .post(url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("failed to call Strata admin RPC: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Strata admin RPC returned non-success status: {e}"))?
        .json()
        .await
        .map_err(|e| format!("failed to decode Strata admin RPC response: {e}"))?;

    if let Some(error) = response.error {
        return Err(format!(
            "Strata admin RPC returned error {}: {}",
            error.code, error.message
        ));
    }

    let result = response
        .result
        .ok_or_else(|| "Strata admin RPC response missing result".to_string())?;
    let admin_keys = extract_admin_keys(&result)?;
    let eligible = admin_keys.iter().any(|key| key == &signer);

    Ok(AuthorityEligibility {
        authority: "strata_admin".to_string(),
        eligible,
    })
}

fn extract_admin_keys(snapshot: &Value) -> Result<Vec<CompressedPublicKey>, String> {
    const KEY_PATHS: [&str; 4] = [
        "/threshold_config/keys",
        "/strata_administrator/keys",
        "/authorities/strata_administrator/config/keys",
        "/authorities/StrataAdministrator/config/keys",
    ];

    for path in KEY_PATHS {
        if let Some(keys) = snapshot.pointer(path).and_then(Value::as_array) {
            return keys
                .iter()
                .map(parse_signer_from_value)
                .collect::<Result<Vec<_>, _>>();
        }
    }

    Err("failed to locate Strata administrator key set in RPC result".to_string())
}

fn parse_signer_from_value(value: &Value) -> Result<CompressedPublicKey, String> {
    if let Some(key) = value.as_str() {
        return normalize_signer_pubkey(key);
    }

    let maybe_key = value
        .as_object()
        .and_then(|obj| {
            obj.get("key")
                .or_else(|| obj.get("pubkey"))
                .or_else(|| obj.get("compressed_pubkey"))
        })
        .and_then(Value::as_str);

    match maybe_key {
        Some(key) => normalize_signer_pubkey(key),
        None => {
            Err("admin key must be a hex string or an object containing key/pubkey".to_string())
        }
    }
}

fn normalize_signer_pubkey(pubkey_hex: &str) -> Result<CompressedPublicKey, String> {
    let trimmed = pubkey_hex
        .trim()
        .strip_prefix("0x")
        .unwrap_or(pubkey_hex.trim());
    let bytes = hex::decode(trimmed).map_err(|e| format!("invalid signer pubkey hex: {e}"))?;
    let parsed =
        PublicKey::from_slice(&bytes).map_err(|e| format!("invalid secp256k1 public key: {e}"))?;
    Ok(CompressedPublicKey::from(parsed))
}

#[cfg(test)]
mod tests {
    use super::{extract_admin_keys, normalize_signer_pubkey};
    use serde_json::json;

    #[test]
    fn normalizes_compressed_key_hex() {
        let compressed = "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5";
        let key = normalize_signer_pubkey(compressed).expect("must parse");
        assert_eq!(hex::encode(key.serialize()), compressed);
    }

    #[test]
    fn normalizes_uncompressed_key_hex_to_compressed() {
        let uncompressed = "04c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee51ae168fea63dc339a3c58419466ceaee\
            f7f632653266d0e1236431a950cfe52a";
        let key = normalize_signer_pubkey(uncompressed).expect("must parse");
        assert_eq!(
            hex::encode(key.serialize()),
            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5"
        );
    }

    #[test]
    fn extracts_admin_keys_from_authorities_path() {
        let snapshot = json!({
            "authorities": {
                "strata_administrator": {
                    "config": {
                        "keys": [
                            "02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5"
                        ]
                    }
                }
            }
        });
        let keys = extract_admin_keys(&snapshot).expect("must extract keys");
        assert_eq!(keys.len(), 1);
    }
}
