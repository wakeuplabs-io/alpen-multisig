//! [`FeeEstimator`] implementation backed by the Electrum protocol (M2).
//!
//! Uses `bdk_electrum::electrum_client::{Client, ElectrumApi}` — the same dependency
//! used by wallet sync. Every call goes through `tokio::task::spawn_blocking` because
//! the Electrum client is blocking I/O, mirroring `wallet_service.rs::do_sync`.

use async_trait::async_trait;

use crate::application::fee_estimation::{FeeEstimateError, FeeEstimator, FeeSource};

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

    fn connect(&self) -> Result<bdk_electrum::electrum_client::Client, FeeEstimateError> {
        bdk_electrum::electrum_client::Client::new(&self.electrum_url)
            .map_err(|e| FeeEstimateError::Unavailable(e.to_string()))
    }
}

#[async_trait]
impl FeeEstimator for ElectrumFeeEstimator {
    fn source(&self) -> FeeSource {
        FeeSource::Electrum
    }

    async fn estimate_sat_per_kvb(&self, target: u16) -> Result<u64, FeeEstimateError> {
        use bdk_electrum::electrum_client::ElectrumApi;

        let url = self.electrum_url.clone();
        let client = self.connect()?;

        tokio::task::spawn_blocking(move || {
            let btc_per_kb = client
                .estimate_fee(target as usize)
                .map_err(|e| FeeEstimateError::Unavailable(e.to_string()))?;

            // Electrum returns -1.0 when no estimate is available.
            if btc_per_kb < 0.0 {
                return Err(FeeEstimateError::NoEstimate {
                    target,
                    reason: format!("Electrum ({url}) returned -1 (no estimate)"),
                });
            }

            // Convert BTC/kB → sat/kvB (ceil), identical formula to Bitcoin Core BTC/kvB.
            // Both use the same denominator (kilo-virtual-bytes); the only difference is that
            // Core reports BTC/kvB and Electrum reports BTC/kB, but for fee estimation
            // purposes kB ≈ kvB (virtual bytes ≈ stripped bytes for typical txs).
            let sat_per_kvb = (btc_per_kb * 1e8).ceil() as u64;
            Ok(sat_per_kvb)
        })
        .await
        .map_err(|e| FeeEstimateError::Unavailable(format!("spawn_blocking panic: {e}")))?
    }

    async fn min_relay_sat_per_kvb(&self) -> Result<u64, FeeEstimateError> {
        use bdk_electrum::electrum_client::ElectrumApi;

        let client = self.connect()?;

        tokio::task::spawn_blocking(move || {
            let btc_per_kb = client
                .relay_fee()
                .map_err(|e| FeeEstimateError::Unavailable(e.to_string()))?;

            let sat_per_kvb = (btc_per_kb * 1e8).ceil() as u64;
            Ok(sat_per_kvb)
        })
        .await
        .map_err(|e| FeeEstimateError::Unavailable(format!("spawn_blocking panic: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_is_electrum() {
        let est = ElectrumFeeEstimator::new("tcp://127.0.0.1:50001");
        assert_eq!(est.source(), FeeSource::Electrum);
    }

    #[tokio::test]
    async fn unreachable_server_maps_to_unavailable() {
        // Port 1 is conventionally unused — connection should fail immediately.
        let est = ElectrumFeeEstimator::new("tcp://127.0.0.1:1");
        let err = est.estimate_sat_per_kvb(6).await.unwrap_err();
        assert!(
            matches!(err, FeeEstimateError::Unavailable(_)),
            "unexpected: {err:?}"
        );
    }
}
