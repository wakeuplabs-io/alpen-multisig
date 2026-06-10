//! [`FeeEstimator`] implementation backed by the Bitcoin node JSON-RPC.

use std::sync::Arc;

use async_trait::async_trait;

use crate::application::fee_estimation::{FeeEstimateError, FeeEstimator, FeeSource};
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
    fn source(&self) -> FeeSource {
        FeeSource::Node
    }

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
    use async_trait::async_trait;
    use bitcoin::Transaction;

    struct FailingRpc;

    #[async_trait]
    impl BitcoinRpcClient for FailingRpc {
        async fn send_raw_transaction(&self, _: &str) -> Result<String, String> {
            unimplemented!()
        }
        async fn get_transaction_confirmations(&self, _: &str) -> Result<u32, String> {
            unimplemented!()
        }
        async fn estimate_smart_fee_sat_per_kvb(&self, _: u16) -> Result<u64, String> {
            Err("connection refused".to_string())
        }
        async fn min_relay_sat_per_kvb(&self) -> Result<u64, String> {
            Err("connection refused".to_string())
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

    /// The only adapter logic: RPC `Err(String)` maps to `FeeEstimateError::Unavailable`,
    /// and the estimator identifies itself as the Node source.
    #[tokio::test]
    async fn rpc_error_maps_to_unavailable_and_source_is_node() {
        let estimator = NodeFeeEstimator::new(Arc::new(FailingRpc));
        assert_eq!(estimator.source(), FeeSource::Node);
        let err = estimator.estimate_sat_per_kvb(6).await.unwrap_err();
        assert!(
            matches!(err, FeeEstimateError::Unavailable(_)),
            "unexpected: {err:?}"
        );
    }
}
