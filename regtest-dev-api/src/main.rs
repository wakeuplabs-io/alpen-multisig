use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
struct Rpc {
    url: String,
    wallet_url: String,
    user: String,
    pass: String,
    client: reqwest::Client,
}

impl Rpc {
    fn new(url: String, user: String, pass: String, wallet: String) -> Self {
        let wallet_url = format!("{}/wallet/{}", url.trim_end_matches('/'), wallet);
        Self {
            url,
            wallet_url,
            user,
            pass,
            client: reqwest::Client::new(),
        }
    }

    async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.call_url(&self.url.clone(), method, params).await
    }

    async fn call_wallet(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        self.call_url(&self.wallet_url.clone(), method, params)
            .await
    }

    async fn call_url(
        &self,
        url: &str,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let body = serde_json::json!({ "jsonrpc": "1.0", "method": method, "params": params });
        let res = self
            .client
            .post(url)
            .basic_auth(&self.user, Some(&self.pass))
            .json(&body)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if let Some(err) = res.get("error").filter(|e| !e.is_null()) {
            anyhow::bail!("RPC error: {err}");
        }
        Ok(res["result"].clone())
    }

    // Resolves the wallet to use: env var > first loaded wallet.
    async fn resolve_wallet(url: &str, user: &str, pass: &str) -> anyhow::Result<String> {
        if let Ok(w) = std::env::var("BITCOIN_RPC_WALLET") {
            return Ok(w);
        }
        let probe = Rpc::new(
            url.to_string(),
            user.to_string(),
            pass.to_string(),
            String::new(),
        );
        let wallets = probe.call("listwallets", serde_json::json!([])).await?;
        let first = wallets
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("no wallets loaded in bitcoind"))?
            .to_string();
        println!("using wallet: {first}");
        Ok(first)
    }
}

// --- Mine ---

#[derive(Deserialize)]
struct MineQuery {
    count: Option<u32>,
}

#[derive(Serialize)]
struct MineResponse {
    blocks_mined: u32,
    block_hashes: Vec<String>,
}

async fn mine(
    State(rpc): State<Arc<Rpc>>,
    Query(q): Query<MineQuery>,
) -> Result<Json<MineResponse>, (StatusCode, String)> {
    let count = q.count.unwrap_or(1).min(200);
    let addr = rpc
        .call_wallet("getnewaddress", serde_json::json!([]))
        .await
        .map_err(rpc_err)?
        .as_str()
        .ok_or_else(|| rpc_err(anyhow::anyhow!("getnewaddress returned non-string")))?
        .to_string();

    let hashes = rpc
        .call("generatetoaddress", serde_json::json!([count, addr]))
        .await
        .map_err(rpc_err)?;

    let block_hashes: Vec<String> =
        serde_json::from_value(hashes).map_err(|e| rpc_err(e.into()))?;

    Ok(Json(MineResponse {
        blocks_mined: block_hashes.len() as u32,
        block_hashes,
    }))
}

// --- Faucet ---

#[derive(Deserialize)]
struct FaucetRequest {
    address: String,
    amount_btc: f64,
}

#[derive(Serialize)]
struct FaucetResponse {
    txid: String,
    block_hash: String,
}

async fn faucet(
    State(rpc): State<Arc<Rpc>>,
    Json(req): Json<FaucetRequest>,
) -> Result<Json<FaucetResponse>, (StatusCode, String)> {
    let txid = rpc
        .call_wallet(
            "sendtoaddress",
            serde_json::json!([req.address, req.amount_btc]),
        )
        .await
        .map_err(rpc_err)?
        .as_str()
        .ok_or_else(|| rpc_err(anyhow::anyhow!("sendtoaddress returned non-string")))?
        .to_string();

    let miner = rpc
        .call_wallet("getnewaddress", serde_json::json!([]))
        .await
        .map_err(rpc_err)?
        .as_str()
        .ok_or_else(|| rpc_err(anyhow::anyhow!("getnewaddress returned non-string")))?
        .to_string();

    let hashes = rpc
        .call("generatetoaddress", serde_json::json!([1, miner]))
        .await
        .map_err(rpc_err)?;

    let block_hash = hashes[0]
        .as_str()
        .ok_or_else(|| rpc_err(anyhow::anyhow!("no block hash")))?
        .to_string();

    Ok(Json(FaucetResponse { txid, block_hash }))
}

// --- Main ---

fn rpc_err(e: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

#[tokio::main]
async fn main() {
    let url = std::env::var("BITCOIN_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:18443".into());
    let user = std::env::var("BITCOIN_RPC_USER").unwrap_or_else(|_| "user".into());
    let pass = std::env::var("BITCOIN_RPC_PASS").unwrap_or_else(|_| "password".into());

    let wallet = Rpc::resolve_wallet(&url, &user, &pass)
        .await
        .expect("failed to resolve bitcoin wallet");

    let rpc = Arc::new(Rpc::new(url, user, pass, wallet));

    let port: u16 = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3001);

    let app = Router::new()
        .route("/mine", post(mine))
        .route("/faucet", post(faucet))
        .with_state(rpc);

    let addr = format!("0.0.0.0:{port}");
    println!("regtest-dev-api listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
