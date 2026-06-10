//! Fee estimation port and preset-derivation service.
//!
//! `FeeEstimator` is the abstraction seam; infrastructure modules implement it.
//! `FeeEstimationService` orchestrates sources in priority order and derives the
//! three presets (Slow / Medium / Fast) with security margins and step rounding.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use crate::domain::fee_rate::{FeeRate, FALLBACK_MIN_RELAY_SAT_PER_KVB, MAX_FEE_RATE_SAT_PER_KVB};

// ---------------------------------------------------------------------------
// Confirmation targets
// ---------------------------------------------------------------------------

/// The three confirmation-speed tiers defined by the functional spec §3.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationTarget {
    /// Next block (~10 min, highest priority).
    Fast,
    /// ~6 blocks (~1 h, balanced — the default preset).
    Medium,
    /// ~12 blocks (~2 h, lowest fee).
    Slow,
}

impl ConfirmationTarget {
    /// Confirmation target in blocks for `estimatesmartfee`.
    pub fn blocks(self) -> u16 {
        match self {
            Self::Fast => 1,
            Self::Medium => 6,
            Self::Slow => 12,
        }
    }

    /// Security margin percentage (functional spec §3.2): Fast +20%, Medium +10%, Slow +5%.
    pub fn margin_pct(self) -> u64 {
        match self {
            Self::Fast => 20,
            Self::Medium => 10,
            Self::Slow => 5,
        }
    }
}

// ---------------------------------------------------------------------------
// FeeEstimator port
// ---------------------------------------------------------------------------

/// Error from a single fee estimation source.
#[derive(Debug, thiserror::Error)]
pub enum FeeEstimateError {
    #[error("fee source unavailable: {0}")]
    Unavailable(String),
    #[error("fee source returned no estimate for target {target} blocks: {reason}")]
    NoEstimate { target: u16, reason: String },
}

/// One fee source (node RPC or Electrum). Returns **raw** rates without margin.
#[async_trait]
pub trait FeeEstimator: Send + Sync {
    /// Raw estimate in sat/kvB for the given confirmation target (blocks).
    async fn estimate_sat_per_kvb(&self, target: u16) -> Result<u64, FeeEstimateError>;

    /// Minimum relay fee in sat/kvB.
    async fn min_relay_sat_per_kvb(&self) -> Result<u64, FeeEstimateError>;
}

// ---------------------------------------------------------------------------
// FeeSource — surfaced to the UI so it can show warnings
// ---------------------------------------------------------------------------

/// Where the presets came from — passed verbatim to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeeSource {
    /// Bitcoin node `estimatesmartfee`.
    Node,
    /// Electrum `blockchain.estimatefee` (M2).
    Electrum,
    /// Previous successful estimate served from the in-memory cache (M2).
    Cached,
    /// Static fallback: presets derived from the minimum relay fee when all live sources fail.
    Fallback,
}

// ---------------------------------------------------------------------------
// FeePreset / FeePresets
// ---------------------------------------------------------------------------

/// A single derived preset (margin applied, rounded up to UI step, clamped).
#[derive(Debug, Clone, Copy)]
pub struct FeePreset {
    pub rate: FeeRate,
    pub target_blocks: u16,
    pub margin_pct: u64,
}

/// All three presets plus metadata for display and IPC serialization.
#[derive(Debug, Clone, Copy)]
pub struct FeePresets {
    pub fast: FeePreset,
    pub medium: FeePreset,
    pub slow: FeePreset,
    pub min_relay_sat_per_kvb: u64,
    pub source: FeeSource,
    /// Unix milliseconds when the underlying estimate was taken.
    pub estimated_at_ms: u64,
}

// ---------------------------------------------------------------------------
// Preset derivation (pure — pub(crate) for tests)
// ---------------------------------------------------------------------------

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Derive a preset for one target: apply margin, round up to step, clamp.
/// Exposed as `pub(crate)` for unit-testing the policy in isolation.
pub(crate) fn derive_preset(raw_sat_per_kvb: u64, target: ConfirmationTarget) -> FeePreset {
    let base = FeeRate::from_raw_clamped(raw_sat_per_kvb);
    let rate = base.with_margin_pct(target.margin_pct()).round_up_to_step();
    FeePreset {
        rate,
        target_blocks: target.blocks(),
        margin_pct: target.margin_pct(),
    }
}

/// Enforce monotonicity: slow ≤ medium ≤ fast.
///
/// `estimatesmartfee` can return inverted rates during a congestion plateau
/// (all three targets report the same high rate). Rather than producing a
/// nonsensical ladder, raise the lower preset to match.
pub(crate) fn enforce_monotonicity(slow: &mut u64, medium: &mut u64, fast: &mut u64) {
    if *medium < *slow {
        *medium = *slow;
    }
    if *fast < *medium {
        *fast = *medium;
    }
}

/// Clamp a raw rate to [min_relay, MAX], then derive a preset.
pub(crate) fn derive_preset_clamped(
    raw_sat_per_kvb: u64,
    min_relay: u64,
    target: ConfirmationTarget,
) -> FeePreset {
    let clamped = raw_sat_per_kvb.max(min_relay).min(MAX_FEE_RATE_SAT_PER_KVB);
    derive_preset(clamped, target)
}

// ---------------------------------------------------------------------------
// FeeEstimationService
// ---------------------------------------------------------------------------

/// Orchestrates fee sources in priority order and produces presets.
///
/// The service is **infallible** (returns `FeePresets` not `Result`): when all
/// live sources fail it falls back to static presets derived from the minimum
/// relay fee, with `source == Fallback`. The UI surfaces this with a warning.
pub struct FeeEstimationService {
    estimators: Vec<Arc<dyn FeeEstimator>>,
}

impl FeeEstimationService {
    pub fn new(estimators: Vec<Arc<dyn FeeEstimator>>) -> Self {
        Self { estimators }
    }

    pub async fn presets(&self) -> FeePresets {
        let now = now_ms();

        for estimator in &self.estimators {
            if let Some(presets) = self.try_estimator(estimator.as_ref(), now).await {
                return presets;
            }
        }

        self.fallback_presets(now)
    }

    async fn try_estimator(&self, estimator: &dyn FeeEstimator, now: u64) -> Option<FeePresets> {
        // All-or-nothing: require all three targets to succeed.
        let fast_raw = estimator
            .estimate_sat_per_kvb(ConfirmationTarget::Fast.blocks())
            .await
            .ok()?;
        let medium_raw = estimator
            .estimate_sat_per_kvb(ConfirmationTarget::Medium.blocks())
            .await
            .ok()?;
        let slow_raw = estimator
            .estimate_sat_per_kvb(ConfirmationTarget::Slow.blocks())
            .await
            .ok()?;

        let min_relay = estimator
            .min_relay_sat_per_kvb()
            .await
            .unwrap_or(FALLBACK_MIN_RELAY_SAT_PER_KVB);

        let mut fast = fast_raw;
        let mut medium = medium_raw;
        let mut slow = slow_raw;
        enforce_monotonicity(&mut slow, &mut medium, &mut fast);

        let source = FeeSource::Node; // overridden by named implementations in M2

        Some(FeePresets {
            fast: derive_preset_clamped(fast, min_relay, ConfirmationTarget::Fast),
            medium: derive_preset_clamped(medium, min_relay, ConfirmationTarget::Medium),
            slow: derive_preset_clamped(slow, min_relay, ConfirmationTarget::Slow),
            min_relay_sat_per_kvb: min_relay,
            source,
            estimated_at_ms: now,
        })
    }

    fn fallback_presets(&self, now: u64) -> FeePresets {
        let min_relay = FALLBACK_MIN_RELAY_SAT_PER_KVB;
        FeePresets {
            fast: derive_preset(min_relay, ConfirmationTarget::Fast),
            medium: derive_preset(min_relay, ConfirmationTarget::Medium),
            slow: derive_preset(min_relay, ConfirmationTarget::Slow),
            min_relay_sat_per_kvb: min_relay,
            source: FeeSource::Fallback,
            estimated_at_ms: now,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // -----------------------------------------------------------------------
    // Mock estimator builder
    // -----------------------------------------------------------------------

    struct MockEstimator {
        /// Responses per target block count (1, 6, 12). `None` = error.
        responses: std::collections::HashMap<u16, Option<u64>>,
        min_relay: Option<u64>,
        /// Track call counts for assertions.
        calls: Mutex<Vec<u16>>,
    }

    impl MockEstimator {
        fn new(fast: Option<u64>, medium: Option<u64>, slow: Option<u64>) -> Self {
            let mut responses = std::collections::HashMap::new();
            responses.insert(1, fast);
            responses.insert(6, medium);
            responses.insert(12, slow);
            Self {
                responses,
                min_relay: Some(FALLBACK_MIN_RELAY_SAT_PER_KVB),
                calls: Mutex::new(vec![]),
            }
        }

        fn with_min_relay(mut self, min_relay: Option<u64>) -> Self {
            self.min_relay = min_relay;
            self
        }
    }

    #[async_trait]
    impl FeeEstimator for MockEstimator {
        async fn estimate_sat_per_kvb(&self, target: u16) -> Result<u64, FeeEstimateError> {
            self.calls.lock().unwrap().push(target);
            match self.responses.get(&target).copied().flatten() {
                Some(v) => Ok(v),
                None => Err(FeeEstimateError::NoEstimate {
                    target,
                    reason: "mock unavailable".to_string(),
                }),
            }
        }

        async fn min_relay_sat_per_kvb(&self) -> Result<u64, FeeEstimateError> {
            self.min_relay
                .map(Ok)
                .unwrap_or_else(|| Err(FeeEstimateError::Unavailable("mock".to_string())))
        }
    }

    fn svc(estimators: Vec<Arc<dyn FeeEstimator>>) -> FeeEstimationService {
        FeeEstimationService::new(estimators)
    }

    // -----------------------------------------------------------------------
    // derive_preset / enforce_monotonicity (pure unit tests)
    // -----------------------------------------------------------------------

    #[test]
    fn derive_preset_applies_margin_and_rounds_up_to_step() {
        // 1_000 sat/kvB fast (+20%) = 1_200 → already a step → 1_200
        let p = derive_preset(1_000, ConfirmationTarget::Fast);
        assert_eq!(p.rate.sat_per_kvb(), 1_200);
        assert_eq!(p.margin_pct, 20);
        assert_eq!(p.target_blocks, 1);
    }

    #[test]
    fn derive_preset_medium_margin_and_step() {
        // 1_000 +10% = 1_100 → step exact → 1_100
        let p = derive_preset(1_000, ConfirmationTarget::Medium);
        assert_eq!(p.rate.sat_per_kvb(), 1_100);
    }

    #[test]
    fn derive_preset_slow_margin_rounds_up() {
        // 1_000 +5% = 1_050 → step 100 → ceil → 1_100
        let p = derive_preset(1_000, ConfirmationTarget::Slow);
        assert_eq!(p.rate.sat_per_kvb(), 1_100);
    }

    #[test]
    fn enforce_monotonicity_raises_medium_when_below_slow() {
        let mut slow = 2_000u64;
        let mut medium = 1_500u64; // inverted: medium < slow
        let mut fast = 3_000u64;
        enforce_monotonicity(&mut slow, &mut medium, &mut fast);
        assert_eq!(medium, 2_000); // raised to slow
        assert_eq!(fast, 3_000); // unchanged
    }

    #[test]
    fn enforce_monotonicity_raises_fast_when_below_medium() {
        let mut slow = 1_000u64;
        let mut medium = 2_000u64;
        let mut fast = 1_500u64; // inverted
        enforce_monotonicity(&mut slow, &mut medium, &mut fast);
        assert_eq!(fast, 2_000); // raised to medium
    }

    #[test]
    fn enforce_monotonicity_all_equal_unchanged() {
        let mut slow = 1_500u64;
        let mut medium = 1_500u64;
        let mut fast = 1_500u64;
        enforce_monotonicity(&mut slow, &mut medium, &mut fast);
        assert_eq!((slow, medium, fast), (1_500, 1_500, 1_500));
    }

    // -----------------------------------------------------------------------
    // FeeEstimationService — node succeeds
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn node_succeeds_source_is_node_and_margins_correct() {
        let mock = Arc::new(MockEstimator::new(Some(1_000), Some(1_000), Some(1_000)));
        let presets = svc(vec![mock]).presets().await;
        assert_eq!(presets.source, FeeSource::Node);
        assert_eq!(presets.fast.margin_pct, 20);
        assert_eq!(presets.medium.margin_pct, 10);
        assert_eq!(presets.slow.margin_pct, 5);
        // fast: 1_000 +20% = 1_200; medium: +10% = 1_100; slow: +5% = 1_050 → step 1_100
        assert_eq!(presets.fast.rate.sat_per_kvb(), 1_200);
        assert_eq!(presets.medium.rate.sat_per_kvb(), 1_100);
        assert_eq!(presets.slow.rate.sat_per_kvb(), 1_100);
    }

    #[tokio::test]
    async fn monotonicity_enforced_when_node_returns_inverted_estimates() {
        // Node returns slow > medium (plateau inversion)
        let mock = Arc::new(MockEstimator::new(
            Some(3_000), // fast
            Some(1_000), // medium < slow (inverted)
            Some(2_000), // slow
        ));
        let presets = svc(vec![mock]).presets().await;
        // After monotonicity: slow=2_000, medium=max(1_000,2_000)=2_000, fast=3_000
        assert!(
            presets.slow.rate.sat_per_kvb() <= presets.medium.rate.sat_per_kvb(),
            "slow > medium: slow={} medium={}",
            presets.slow.rate.sat_per_kvb(),
            presets.medium.rate.sat_per_kvb()
        );
        assert!(
            presets.medium.rate.sat_per_kvb() <= presets.fast.rate.sat_per_kvb(),
            "medium > fast"
        );
    }

    #[tokio::test]
    async fn preset_below_min_relay_is_clamped_up() {
        // Raw estimate 500 sat/kvB, min_relay 1_000 → clamped to 1_000 before margin
        let mock = Arc::new(
            MockEstimator::new(Some(500), Some(500), Some(500)).with_min_relay(Some(1_000)),
        );
        let presets = svc(vec![mock]).presets().await;
        // After clamp to 1_000: fast +20% = 1_200
        assert_eq!(presets.fast.rate.sat_per_kvb(), 1_200);
    }

    #[tokio::test]
    async fn preset_above_max_is_clamped_down() {
        let mock = Arc::new(MockEstimator::new(
            Some(MAX_FEE_RATE_SAT_PER_KVB),
            Some(MAX_FEE_RATE_SAT_PER_KVB),
            Some(MAX_FEE_RATE_SAT_PER_KVB),
        ));
        let presets = svc(vec![mock]).presets().await;
        assert_eq!(presets.fast.rate.sat_per_kvb(), MAX_FEE_RATE_SAT_PER_KVB);
    }

    // -----------------------------------------------------------------------
    // All-or-nothing: one target failing skips the source entirely
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn one_failing_target_skips_source_and_falls_back() {
        // Medium target returns None → source must be skipped → Fallback
        let mock = Arc::new(MockEstimator::new(
            Some(1_000), // fast ok
            None,        // medium fails
            Some(1_000), // slow ok
        ));
        let presets = svc(vec![mock]).presets().await;
        assert_eq!(presets.source, FeeSource::Fallback);
    }

    // -----------------------------------------------------------------------
    // Fallback when all sources fail
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn all_sources_fail_produces_fallback_presets() {
        let mock = Arc::new(MockEstimator::new(None, None, None));
        let presets = svc(vec![mock]).presets().await;
        assert_eq!(presets.source, FeeSource::Fallback);
        // fallback base = FALLBACK_MIN_RELAY (1_000):
        // fast +20% = 1_200; medium +10% = 1_100; slow +5% = 1_050 → step 1_100
        assert_eq!(presets.fast.rate.sat_per_kvb(), 1_200);
        assert_eq!(presets.medium.rate.sat_per_kvb(), 1_100);
        assert_eq!(presets.slow.rate.sat_per_kvb(), 1_100);
        assert_eq!(
            presets.min_relay_sat_per_kvb,
            FALLBACK_MIN_RELAY_SAT_PER_KVB
        );
    }

    #[tokio::test]
    async fn fallback_with_no_estimators_completes_without_panic() {
        let presets = svc(vec![]).presets().await;
        assert_eq!(presets.source, FeeSource::Fallback);
    }

    // -----------------------------------------------------------------------
    // min_relay failure does not abort preset derivation
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn min_relay_failure_uses_fallback_min_relay_but_estimates_succeed() {
        let mock = Arc::new(
            MockEstimator::new(Some(2_000), Some(1_500), Some(1_200)).with_min_relay(None),
        );
        let presets = svc(vec![mock]).presets().await;
        // source should still be Node (estimates succeeded)
        assert_eq!(presets.source, FeeSource::Node);
        // min_relay falls back to FALLBACK_MIN_RELAY_SAT_PER_KVB
        assert_eq!(
            presets.min_relay_sat_per_kvb,
            FALLBACK_MIN_RELAY_SAT_PER_KVB
        );
    }

    // -----------------------------------------------------------------------
    // presets() never returns Err (type-level: it returns FeePresets directly)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn presets_always_returns_a_value_even_on_total_failure() {
        let mock = Arc::new(MockEstimator::new(None, None, None));
        // If this compiles and doesn't panic, we're good — the return type is FeePresets.
        let _presets: FeePresets = svc(vec![mock]).presets().await;
    }
}
