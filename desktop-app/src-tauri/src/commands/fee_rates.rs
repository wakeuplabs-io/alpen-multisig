//! IPC boundary for fee rate estimation — DTO mapping only, no policy.

use std::sync::Arc;

use desktop_app::application::fee_estimation::{FeeEstimationService, FeePresets, FeeSource};
use desktop_app::domain::fee_constants::{
    COMMIT_DUST_SATS, COMMIT_TX_VBYTES_ESTIMATE, REVEAL_TX_VBYTES,
};
use desktop_app::domain::fee_rate::MAX_FEE_RATE_SAT_PER_KVB;
use desktop_app::infrastructure::bitcoin_rpc::HttpBitcoinRpcClient;
use desktop_app::infrastructure::node_config_store::NodeConfigState;
use desktop_app::infrastructure::node_fee_estimator::NodeFeeEstimator;

/// Response DTO for a single preset.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeePresetDto {
    pub sat_per_kvb: u64,
    pub target_blocks: u16,
    pub margin_pct: u64,
}

/// Response DTO returned by `fee_rates_estimate`.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeRatesDto {
    pub fast: FeePresetDto,
    pub medium: FeePresetDto,
    pub slow: FeePresetDto,
    pub min_relay_sat_per_kvb: u64,
    /// Hard upper bound for custom fee input. Frontend must not hardcode this.
    pub max_sat_per_kvb: u64,
    /// Serialized as "node" | "electrum" | "cached" | "fallback".
    pub source: FeeSource,
    /// Unix ms when the underlying estimate was taken.
    pub estimated_at_ms: u64,
    /// Reveal transaction vsize (from `REVEAL_TX_VBYTES`). Used for local fee recompute in UI.
    pub reveal_vbytes: u64,
    /// Commit transaction vsize estimate. Used for total-fee display in UI.
    pub commit_vbytes_estimate: u64,
    /// Dust amount locked in the commit output (SPS-51).
    pub commit_dust_sats: u64,
}

fn to_dto(presets: FeePresets) -> FeeRatesDto {
    FeeRatesDto {
        fast: FeePresetDto {
            sat_per_kvb: presets.fast.rate.sat_per_kvb(),
            target_blocks: presets.fast.target_blocks,
            margin_pct: presets.fast.margin_pct,
        },
        medium: FeePresetDto {
            sat_per_kvb: presets.medium.rate.sat_per_kvb(),
            target_blocks: presets.medium.target_blocks,
            margin_pct: presets.medium.margin_pct,
        },
        slow: FeePresetDto {
            sat_per_kvb: presets.slow.rate.sat_per_kvb(),
            target_blocks: presets.slow.target_blocks,
            margin_pct: presets.slow.margin_pct,
        },
        min_relay_sat_per_kvb: presets.min_relay_sat_per_kvb,
        max_sat_per_kvb: MAX_FEE_RATE_SAT_PER_KVB,
        source: presets.source,
        estimated_at_ms: presets.estimated_at_ms,
        reveal_vbytes: REVEAL_TX_VBYTES,
        commit_vbytes_estimate: COMMIT_TX_VBYTES_ESTIMATE,
        commit_dust_sats: COMMIT_DUST_SATS,
    }
}

/// Estimate fee presets for governance broadcast (and wallet Send in later phases).
///
/// Read-only: requires neither an active wallet session nor signing capability.
#[tauri::command]
pub async fn fee_rates_estimate(
    node_config: tauri::State<'_, NodeConfigState>,
) -> Result<FeeRatesDto, String> {
    let cfg = node_config
        .0
        .read()
        .map_err(|e| format!("lock error: {e}"))?
        .clone();

    let rpc = Arc::new(HttpBitcoinRpcClient::new(
        cfg.btc_rpc_url(),
        cfg.btc_rpc_user(),
        cfg.btc_rpc_pass(),
    ));
    let node_estimator = Arc::new(NodeFeeEstimator::new(rpc));
    let service = FeeEstimationService::new(vec![node_estimator]);
    let presets = service.presets().await;
    Ok(to_dto(presets))
}

#[cfg(test)]
mod tests {
    use super::*;
    use desktop_app::application::fee_estimation::{ConfirmationTarget, FeePreset, FeePresets};
    use desktop_app::domain::fee_rate::{FeeRate, FALLBACK_MIN_RELAY_SAT_PER_KVB};

    fn make_presets(source: FeeSource) -> FeePresets {
        let fast_rate = FeeRate::new(1_200, 1_000).unwrap();
        let medium_rate = FeeRate::new(1_100, 1_000).unwrap();
        let slow_rate = FeeRate::new(1_100, 1_000).unwrap();
        FeePresets {
            fast: FeePreset {
                rate: fast_rate,
                target_blocks: ConfirmationTarget::Fast.blocks(),
                margin_pct: ConfirmationTarget::Fast.margin_pct(),
            },
            medium: FeePreset {
                rate: medium_rate,
                target_blocks: ConfirmationTarget::Medium.blocks(),
                margin_pct: ConfirmationTarget::Medium.margin_pct(),
            },
            slow: FeePreset {
                rate: slow_rate,
                target_blocks: ConfirmationTarget::Slow.blocks(),
                margin_pct: ConfirmationTarget::Slow.margin_pct(),
            },
            min_relay_sat_per_kvb: FALLBACK_MIN_RELAY_SAT_PER_KVB,
            source,
            estimated_at_ms: 1_700_000_000_000,
        }
    }

    /// IPC contract: the serialized JSON must use camelCase keys and snake_case
    /// source values — exactly what `feeRatesSchema` on the frontend expects.
    #[test]
    fn dto_serializes_to_camel_case_json_matching_frontend_schema() {
        let json =
            serde_json::to_value(to_dto(make_presets(FeeSource::Node))).expect("serializable");

        assert_eq!(json["fast"]["satPerKvb"], 1_200);
        assert_eq!(json["fast"]["targetBlocks"], 1);
        assert_eq!(json["fast"]["marginPct"], 20);
        assert_eq!(json["medium"]["satPerKvb"], 1_100);
        assert_eq!(json["medium"]["targetBlocks"], 6);
        assert_eq!(json["slow"]["satPerKvb"], 1_100);
        assert_eq!(json["slow"]["targetBlocks"], 12);

        assert_eq!(json["minRelaySatPerKvb"], FALLBACK_MIN_RELAY_SAT_PER_KVB);
        assert_eq!(json["maxSatPerKvb"], MAX_FEE_RATE_SAT_PER_KVB);
        assert_eq!(json["source"], "node");
        assert_eq!(json["estimatedAtMs"], 1_700_000_000_000u64);
        assert_eq!(json["revealVbytes"], REVEAL_TX_VBYTES);
        assert_eq!(json["commitVbytesEstimate"], COMMIT_TX_VBYTES_ESTIMATE);
        assert_eq!(json["commitDustSats"], COMMIT_DUST_SATS);
    }

    #[test]
    fn dto_source_fallback_serializes_as_snake_case_string() {
        let json =
            serde_json::to_value(to_dto(make_presets(FeeSource::Fallback))).expect("serializable");
        assert_eq!(json["source"], "fallback");
    }
}
