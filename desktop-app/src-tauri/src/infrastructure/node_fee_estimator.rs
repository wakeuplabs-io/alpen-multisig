//! [`FeeEstimator`] implementation backed by the Bitcoin node JSON-RPC.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::fee_estimation::{FeeEstimateError, FeeEstimator};
use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;

/// `FeeEstimator` that delegates to the Bitcoin node via [`BitcoinRpcClient`].
pub struct NodeFeeEstimator {
    rpc: Arc<dyn BitcoinRpcClient>,
}

impl NodeFeeEstimator {
    pub fn new(rpc: Arc<dyn BitcoinRpcClient>) -> Self {
        Self { rpc }
    }
}

#[async_trait]
impl FeeEstimator for NodeFeeEstimator {
    async fn estimate_sat_per_kvb(&self, target: u16) -> Result<u64, FeeEstimateError> {
        self.rpc
            .estimate_smart_fee_sat_per_kvb(target)
            .await
            .map_err(FeeEstimateError::Unavailable)
    }

    async fn min_relay_sat_per_kvb(&self) -> Result<u64, FeeEstimateError> {
        self.rpc
            .min_relay_sat_per_kvb()
            .await
            .map_err(FeeEstimateError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::bitcoin_rpc::BitcoinRpcClient;
    use async_trait::async_trait;
    use bitcoin::Transaction;

    struct MockRpc {
        feerate_result: Result<u64, String>,
        min_relay_result: Result<u64, String>,
    }

    #[async_trait]
    impl BitcoinRpcClient for MockRpc {
        async fn send_raw_transaction(&self, _: &str) -> Result<String, String> {
            unimplemented!()
        }
        async fn get_transaction_confirmations(&self, _: &str) -> Result<u32, String> {
            unimplemented!()
        }
        async fn estimate_fee_rate_sats_per_vb(&self, _: u16) -> Result<u64, String> {
            unimplemented!()
        }
        async fn estimate_smart_fee_sat_per_kvb(&self, _: u16) -> Result<u64, String> {
            self.feerate_result.clone()
        }
        async fn min_relay_sat_per_kvb(&self) -> Result<u64, String> {
            self.min_relay_result.clone()
        }
        async fn get_raw_transaction(&self, _: &str) -> Result<Transaction, String> {
            unimplemented!()
        }
        async fn get_block_count(&self) -> Result<u64, String> {
            unimplemented!()
        }
        async fn submit_package(&self, _: &[String]) -> Result<(), String> {
            unimplemented!()
        }
    }

    fn mock_rpc(feerate: Result<u64, String>, min_relay: Result<u64, String>) -> Arc<MockRpc> {
        Arc::new(MockRpc {
            feerate_result: feerate,
            min_relay_result: min_relay,
        })
    }

    #[tokio::test]
    async fn estimate_sat_per_kvb_delegates_to_rpc() {
        let estimator = NodeFeeEstimator::new(mock_rpc(Ok(1_000), Ok(1_000)));
        assert_eq!(estimator.estimate_sat_per_kvb(6).await.unwrap(), 1_000);
    }

    #[tokio::test]
    async fn rpc_error_maps_to_unavailable() {
        let estimator =
            NodeFeeEstimator::new(mock_rpc(Err("connection refused".to_string()), Ok(1_000)));
        let err = estimator.estimate_sat_per_kvb(6).await.unwrap_err();
        assert!(
            matches!(err, FeeEstimateError::Unavailable(_)),
            "unexpected: {err:?}"
        );
    }

    #[tokio::test]
    async fn min_relay_delegates_to_rpc() {
        let estimator = NodeFeeEstimator::new(mock_rpc(Ok(1_000), Ok(1_500)));
        assert_eq!(estimator.min_relay_sat_per_kvb().await.unwrap(), 1_500);
    }
}
