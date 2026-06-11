//! Fee estimation port and preset-derivation service.
//!
//! `FeeEstimator` is the abstraction seam; infrastructure modules implement it.
//! `FeeEstimationService` orchestrates sources in priority order and derives the
//! three presets (Slow / Medium / Fast) with security margins and step rounding.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use crate::domain::fee_rate::{FeeRate, FALLBACK_MIN_RELAY_SAT_PER_KVB, MAX_FEE_RATE_SAT_PER_KVB};

/// Maximum age of a cached estimate before it is discarded (10 minutes).
const CACHE_MAX_AGE_MS: u64 = 10 * 60 * 1_000;

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
    /// Which source this estimator represents — surfaced to the UI verbatim.
    fn source(&self) -> FeeSource;

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

/// Enforce monotonicity: slow ≤ medium ≤ fast. Returns `(slow, medium, fast)`.
///
/// `estimatesmartfee` can return inverted rates during a congestion plateau
/// (all three targets report the same high rate). Rather than producing a
/// nonsensical ladder, raise the lower preset to match.
pub(crate) fn enforce_monotonicity(slow: u64, medium: u64, fast: u64) -> (u64, u64, u64) {
    let medium = medium.max(slow);
    let fast = fast.max(medium);
    (slow, medium, fast)
}

/// Clamp a raw rate to [min_relay, MAX], then derive a preset.
pub(crate) fn derive_preset_clamped(
    raw_sat_per_kvb: u64,
    min_relay: u64,
    target: ConfirmationTarget,
) -> FeePreset {
    let clamped = raw_sat_per_kvb.clamp(min_relay, MAX_FEE_RATE_SAT_PER_KVB);
    derive_preset(clamped, target)
}

// ---------------------------------------------------------------------------
// FeeEstimationService + Tauri managed state
// ---------------------------------------------------------------------------

/// Tauri managed state holding the shared in-memory fee cache (M2).
///
/// Only the cache is managed (registered once at startup so it survives across
/// `fee_rates_estimate` IPC calls); the estimators themselves are rebuilt per
/// call from the **current** node config, so runtime config changes take effect
/// immediately — same as the broadcast path.
#[derive(Default)]
pub struct FeeCacheState(pub Arc<Mutex<Option<FeePresets>>>);

// FeeEstimationService -------------------------------------------------------

/// Orchestrates fee sources in priority order and produces presets.
///
/// The service is **infallible** (returns `FeePresets` not `Result`): when all
/// live sources fail it first tries the in-memory cache (younger than
/// `CACHE_MAX_AGE_MS`), then falls back to static presets derived from the
/// minimum relay fee. The UI surfaces Cached / Fallback with a warning banner.
pub struct FeeEstimationService {
    estimators: Vec<Arc<dyn FeeEstimator>>,
    /// Last successful estimate, shared via [`FeeCacheState`] across calls.
    cache: Arc<Mutex<Option<FeePresets>>>,
}

impl FeeEstimationService {
    pub fn new(estimators: Vec<Arc<dyn FeeEstimator>>) -> Self {
        Self::with_cache(estimators, Arc::default())
    }

    pub fn with_cache(
        estimators: Vec<Arc<dyn FeeEstimator>>,
        cache: Arc<Mutex<Option<FeePresets>>>,
    ) -> Self {
        Self { estimators, cache }
    }

    pub async fn presets(&self) -> FeePresets {
        let now = now_ms();

        for estimator in &self.estimators {
            if let Some(presets) = self.try_estimator(estimator.as_ref(), now).await {
                // Store a fresh copy in the cache for subsequent fallback.
                if let Ok(mut guard) = self.cache.lock() {
                    *guard = Some(presets);
                }
                return presets;
            }
        }

        // All live sources failed — try the in-memory cache.
        if let Some(cached) = self.cached_presets(now) {
            return cached;
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

        let (slow, medium, fast) = enforce_monotonicity(slow_raw, medium_raw, fast_raw);

        Some(FeePresets {
            fast: derive_preset_clamped(fast, min_relay, ConfirmationTarget::Fast),
            medium: derive_preset_clamped(medium, min_relay, ConfirmationTarget::Medium),
            slow: derive_preset_clamped(slow, min_relay, ConfirmationTarget::Slow),
            min_relay_sat_per_kvb: min_relay,
            source: estimator.source(),
            estimated_at_ms: now,
        })
    }

    fn cached_presets(&self, now: u64) -> Option<FeePresets> {
        let guard = self.cache.lock().ok()?;
        let entry = (*guard)?;
        let age_ms = now.saturating_sub(entry.estimated_at_ms);
        if age_ms > CACHE_MAX_AGE_MS {
            return None;
        }
        Some(FeePresets {
            source: FeeSource::Cached,
            ..entry
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

    // -----------------------------------------------------------------------
    // Mock estimator builder
    // -----------------------------------------------------------------------

    struct MockEstimator {
        /// Responses per target block count (1, 6, 12). `None` = error.
        responses: std::collections::HashMap<u16, Option<u64>>,
        min_relay: Option<u64>,
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
            }
        }

        fn with_min_relay(mut self, min_relay: Option<u64>) -> Self {
            self.min_relay = min_relay;
            self
        }
    }

    #[async_trait]
    impl FeeEstimator for MockEstimator {
        fn source(&self) -> FeeSource {
            FeeSource::Node
        }

        async fn estimate_sat_per_kvb(&self, target: u16) -> Result<u64, FeeEstimateError> {
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
        // inverted: medium < slow
        assert_eq!(
            enforce_monotonicity(2_000, 1_500, 3_000),
            (2_000, 2_000, 3_000)
        );
    }

    #[test]
    fn enforce_monotonicity_raises_fast_when_below_medium() {
        // inverted: fast < medium
        assert_eq!(
            enforce_monotonicity(1_000, 2_000, 1_500),
            (1_000, 2_000, 2_000)
        );
    }

    #[test]
    fn enforce_monotonicity_all_equal_unchanged() {
        assert_eq!(
            enforce_monotonicity(1_500, 1_500, 1_500),
            (1_500, 1_500, 1_500)
        );
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

        // No estimators configured at all behaves identically.
        let empty = svc(vec![]).presets().await;
        assert_eq!(empty.source, FeeSource::Fallback);
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
    // Electrum fallback (M2): source ordering
    // -----------------------------------------------------------------------

    struct ElectrumMock(MockEstimator);

    #[async_trait]
    impl FeeEstimator for ElectrumMock {
        fn source(&self) -> FeeSource {
            FeeSource::Electrum
        }

        async fn estimate_sat_per_kvb(&self, target: u16) -> Result<u64, FeeEstimateError> {
            self.0.estimate_sat_per_kvb(target).await
        }

        async fn min_relay_sat_per_kvb(&self) -> Result<u64, FeeEstimateError> {
            self.0.min_relay_sat_per_kvb().await
        }
    }

    fn electrum_mock(
        fast: Option<u64>,
        medium: Option<u64>,
        slow: Option<u64>,
    ) -> Arc<ElectrumMock> {
        Arc::new(ElectrumMock(MockEstimator::new(fast, medium, slow)))
    }

    #[tokio::test]
    async fn node_fails_electrum_succeeds_source_is_electrum() {
        let node = Arc::new(MockEstimator::new(None, None, None));
        let electrum = electrum_mock(Some(1_000), Some(1_000), Some(1_000));
        let node: Arc<dyn FeeEstimator> = node;
        let electrum: Arc<dyn FeeEstimator> = electrum;
        let presets = svc(vec![node, electrum]).presets().await;
        assert_eq!(presets.source, FeeSource::Electrum);
    }

    // -----------------------------------------------------------------------
    // Cache (M2): last success served as Cached while fresh; Fallback when stale
    // -----------------------------------------------------------------------

    /// Estimator whose availability can be flipped between calls.
    struct SwitchableEstimator {
        ok: std::sync::atomic::AtomicBool,
    }

    #[async_trait]
    impl FeeEstimator for SwitchableEstimator {
        fn source(&self) -> FeeSource {
            FeeSource::Node
        }

        async fn estimate_sat_per_kvb(&self, target: u16) -> Result<u64, FeeEstimateError> {
            if self.ok.load(std::sync::atomic::Ordering::Relaxed) {
                Ok(1_000)
            } else {
                Err(FeeEstimateError::NoEstimate {
                    target,
                    reason: "down".to_string(),
                })
            }
        }

        async fn min_relay_sat_per_kvb(&self) -> Result<u64, FeeEstimateError> {
            Ok(FALLBACK_MIN_RELAY_SAT_PER_KVB)
        }
    }

    /// End-to-end cache behavior: a successful estimate populates the cache, and
    /// once all sources go down the same rates are served with source=Cached.
    #[tokio::test]
    async fn cache_serves_last_success_when_sources_go_down() {
        let est = Arc::new(SwitchableEstimator {
            ok: std::sync::atomic::AtomicBool::new(true),
        });
        let svc = FeeEstimationService::new(vec![Arc::clone(&est) as Arc<dyn FeeEstimator>]);

        let live = svc.presets().await;
        assert_eq!(live.source, FeeSource::Node);

        est.ok.store(false, std::sync::atomic::Ordering::Relaxed);
        let cached = svc.presets().await;
        assert_eq!(cached.source, FeeSource::Cached);
        assert_eq!(
            cached.fast.rate.sat_per_kvb(),
            live.fast.rate.sat_per_kvb(),
            "cached presets must carry the rates of the last successful estimate"
        );
    }

    fn make_cached_presets(age_ms: u64) -> FeePresets {
        let ts = now_ms().saturating_sub(age_ms);
        let preset = FeePreset {
            rate: FeeRate::from_raw_clamped(1_000),
            target_blocks: 6,
            margin_pct: 10,
        };
        FeePresets {
            fast: preset,
            medium: preset,
            slow: preset,
            min_relay_sat_per_kvb: FALLBACK_MIN_RELAY_SAT_PER_KVB,
            source: FeeSource::Node,
            estimated_at_ms: ts,
        }
    }

    #[tokio::test]
    async fn all_fail_stale_cache_returns_fallback() {
        let failing: Arc<dyn FeeEstimator> = Arc::new(MockEstimator::new(None, None, None));
        // Cache entry 11 minutes old (stale — beyond CACHE_MAX_AGE_MS)
        let cache = Arc::new(Mutex::new(Some(make_cached_presets(11 * 60 * 1_000))));
        let svc = FeeEstimationService::with_cache(vec![failing], cache);
        let presets = svc.presets().await;
        assert_eq!(presets.source, FeeSource::Fallback);
        assert_eq!(presets.fast.rate.sat_per_kvb(), 1_200);
    }
}
