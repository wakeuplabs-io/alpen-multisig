//! Dev-only handlers — only registered when `DEV_MINE_ENABLED=1`.
//!
//! These endpoints are for regtest/staging environments where the orchestrator
//! runs alongside a local bitcoind. Never expose in production.

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::state::AppState;

const MAX_BLOCKS_PER_REQUEST: u32 = 200;

#[derive(Deserialize)]
pub struct MineParams {
    /// Number of blocks to mine (default: 1, max: 200).
    count: Option<u32>,
}

#[derive(Serialize)]
pub struct MineResult {
    blocks_mined: u32,
    block_hashes: Vec<String>,
}

/// `POST /dev/mine?count=N`
///
/// Mine `count` regtest blocks (default 1). Returns the newly mined block hashes.
/// Only registered when `DEV_MINE_ENABLED=1`.
pub async fn mine_blocks(
    State(state): State<AppState>,
    Query(params): Query<MineParams>,
) -> Result<Json<MineResult>> {
    let count = params.count.unwrap_or(1).min(MAX_BLOCKS_PER_REQUEST);
    let address = state.btc_client.get_new_address().await?;
    let block_hashes = state
        .btc_client
        .generate_to_address(count, &address)
        .await?;
    Ok(Json(MineResult {
        blocks_mined: block_hashes.len() as u32,
        block_hashes,
    }))
}
