//! [`FeeEstimator`] implementation backed by the Electrum protocol (M2).
//!
//! Uses `bdk_electrum::electrum_client::{Client, ElectrumApi}` — the same dependency
//! used by wallet sync. Connection and calls both run inside
//! `tokio::task::spawn_blocking` because the Electrum client is blocking I/O,
//! mirroring `wallet_service.rs::do_sync`.

use async_trait::async_trait;

use crate::application::fee_estimation::{FeeEstimateError, FeeEstimator, FeeSource};

/// Electrum reports fees as BTC/kB; convert to sat/kvB.
/// Returns `None` when the server reports a negative rate (-1 = "no estimate").
///
/// BTC values carry at most 8 decimals, so `btc * 1e8` is always a whole number
/// in decimal — `round()` recovers it exactly, whereas `ceil()` would inflate the
/// result by 1 sat on f64 representation noise (e.g. 0.00001 * 1e8 = 1000.0000000000001).
///
/// kB vs kvB: Core reports BTC/kvB while Electrum reports BTC/kB, but for fee
/// estimation purposes kB ≈ kvB (virtual bytes ≈ stripped bytes for typical txs).
fn btc_per_kb_to_sat_per_kvb(btc_per_kb: f64) -> Option<u64> {
    if btc_per_kb < 0.0 {
        return None;
    }
    Some((btc_per_kb * 1e8).round() as u64)
}

/// `FeeEstimator` that delegates to an Electrum server.
pub struct ElectrumFeeEstimator {
    electrum_url: String,
}

impl ElectrumFeeEstimator {
    pub fn new(electrum_url: impl Into<String>) -> Self {
        Self {
            electrum_url: electrum_url.into(),
        }
    }

    /// Run `f` against a fresh Electrum connection on the blocking pool.
    async fn with_client<T, F>(&self, f: F) -> Result<T, FeeEstimateError>
    where
        T: Send + 'static,
        F: FnOnce(&bdk_electrum::electrum_client::Client) -> Result<T, FeeEstimateError>
            + Send
            + 'static,
    {
        let url = self.electrum_url.clone();
        tokio::task::spawn_blocking(move || {
            let client = bdk_electrum::electrum_client::Client::new(&url)
                .map_err(|e| FeeEstimateError::Unavailable(e.to_string()))?;
            f(&client)
        })
        .await
        .map_err(|e| FeeEstimateError::Unavailable(format!("spawn_blocking panic: {e}")))?
    }
}

#[async_trait]
impl FeeEstimator for ElectrumFeeEstimator {
    fn source(&self) -> FeeSource {
        FeeSource::Electrum
    }

    async fn estimate_sat_per_kvb(&self, target: u16) -> Result<u64, FeeEstimateError> {
        use bdk_electrum::electrum_client::ElectrumApi;

        self.with_client(move |client| {
            let btc_per_kb = client
                .estimate_fee(target as usize)
                .map_err(|e| FeeEstimateError::Unavailable(e.to_string()))?;
            btc_per_kb_to_sat_per_kvb(btc_per_kb).ok_or(FeeEstimateError::NoEstimate {
                target,
                reason: "Electrum returned -1 (no estimate)".to_string(),
            })
        })
        .await
    }

    async fn min_relay_sat_per_kvb(&self) -> Result<u64, FeeEstimateError> {
        use bdk_electrum::electrum_client::ElectrumApi;

        self.with_client(|client| {
            let btc_per_kb = client
                .relay_fee()
                .map_err(|e| FeeEstimateError::Unavailable(e.to_string()))?;
            btc_per_kb_to_sat_per_kvb(btc_per_kb)
                .ok_or_else(|| FeeEstimateError::Unavailable("negative relay fee".to_string()))
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::btc_per_kb_to_sat_per_kvb;

    #[test]
    fn conversion_btc_per_kb_to_sat_per_kvb() {
        // 0.00001 BTC/kB = 1_000 sat/kvB (1 sat/vB) — exact despite f64 noise
        // (0.00001 * 1e8 = 1000.0000000000001; ceil would wrongly give 1_001).
        assert_eq!(btc_per_kb_to_sat_per_kvb(0.00001), Some(1_000));
        assert_eq!(btc_per_kb_to_sat_per_kvb(0.00012345), Some(12_345));
        // -1 = Electrum's "no estimate available"
        assert_eq!(btc_per_kb_to_sat_per_kvb(-1.0), None);
        assert_eq!(btc_per_kb_to_sat_per_kvb(0.0), Some(0));
    }
}
