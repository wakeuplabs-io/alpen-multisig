/// 0.1 sat/vB — the UI increment (100 sat/kvB).
pub const FEE_RATE_STEP_SAT_PER_KVB: u64 = 100;

/// 10,000 sat/vB — hard ceiling from the PRD (10_000_000 sat/kvB).
pub const MAX_FEE_RATE_SAT_PER_KVB: u64 = 10_000_000;

/// 1 sat/vB — static fallback when no relay-fee source is reachable (1_000 sat/kvB).
pub const FALLBACK_MIN_RELAY_SAT_PER_KVB: u64 = 1_000;

/// 10,000 sat/vB — the highest rate a node will accept for **one transaction**.
///
/// `sendrawtransaction` rejects anything above `0.10 BTC/kvB` by default (Core's
/// `-maxfeerate`), and `Psbt::extract_tx` refuses to even build a transaction over
/// 25,000 sat/vB. Numerically the same as [`MAX_FEE_RATE_SAT_PER_KVB`], but a
/// different rule: that one caps the rate an operator may *ask for*, this one caps
/// what a single transaction may *pay*. They come apart in CPFP, where the rate
/// asked for applies to the package while the cap below applies to the child alone
/// (#431) — see `PackageStats::max_package_rate_sat_per_kvb`.
pub const MAX_BROADCAST_SAT_PER_KVB: u64 = 10_000_000;

/// A validated fee rate in satoshis per virtual kilobyte (sat/kvB).
///
/// Unit: 1 sat/vB == 1_000 sat/kvB; the UI step of 0.1 sat/vB == 100 sat/kvB.
/// Invariant: `0 < value <= MAX_FEE_RATE_SAT_PER_KVB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FeeRate(u64);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FeeRateError {
    #[error("fee rate {given} sat/kvB is below the minimum relay fee {min_relay} sat/kvB")]
    BelowMinRelay { given: u64, min_relay: u64 },
    #[error("fee rate {given} sat/kvB exceeds the maximum {max} sat/kvB (10,000 sat/vB)")]
    AboveMax { given: u64, max: u64 },
    #[error("fee rate must be greater than zero")]
    Zero,
}

impl FeeRate {
    /// Construct without validation, clamped to [1, MAX]. Internal use by the estimation service.
    pub(crate) fn from_raw_clamped(sat_per_kvb: u64) -> Self {
        Self(sat_per_kvb.clamp(1, MAX_FEE_RATE_SAT_PER_KVB))
    }

    /// Validate and wrap a raw sat/kvB value.
    ///
    /// `min_relay_sat_per_kvb` is the node's minimum relay fee; typically 1_000 (1 sat/vB).
    pub fn new(sat_per_kvb: u64, min_relay_sat_per_kvb: u64) -> Result<Self, FeeRateError> {
        if sat_per_kvb == 0 {
            return Err(FeeRateError::Zero);
        }
        if sat_per_kvb < min_relay_sat_per_kvb {
            return Err(FeeRateError::BelowMinRelay {
                given: sat_per_kvb,
                min_relay: min_relay_sat_per_kvb,
            });
        }
        if sat_per_kvb > MAX_FEE_RATE_SAT_PER_KVB {
            return Err(FeeRateError::AboveMax {
                given: sat_per_kvb,
                max: MAX_FEE_RATE_SAT_PER_KVB,
            });
        }
        Ok(Self(sat_per_kvb))
    }

    /// Raw sat/kvB value (always > 0).
    pub fn sat_per_kvb(self) -> u64 {
        self.0
    }

    /// Absolute fee in sats for `vbytes` virtual bytes, rounded up (ceiling).
    ///
    /// Formula: `ceil(sat_per_kvb * vbytes / 1000)`.
    pub fn fee_sats(self, vbytes: u64) -> u64 {
        self.0.saturating_mul(vbytes).div_ceil(1_000)
    }

    /// Add a percentage margin, rounding up, saturating at `MAX_FEE_RATE_SAT_PER_KVB`.
    ///
    /// Formula: `ceil(rate * (100 + pct) / 100)`.
    pub fn with_margin_pct(self, pct: u64) -> Self {
        let raw = self.0.saturating_mul(100 + pct).div_ceil(100);
        Self(raw.min(MAX_FEE_RATE_SAT_PER_KVB))
    }

    /// Round up to the next 0.1 sat/vB UI step (100 sat/kvB).
    pub fn round_up_to_step(self) -> Self {
        let raw = self.0.div_ceil(FEE_RATE_STEP_SAT_PER_KVB) * FEE_RATE_STEP_SAT_PER_KVB;
        Self(raw.min(MAX_FEE_RATE_SAT_PER_KVB))
    }

    /// Convert to [`bdk_wallet::bitcoin::FeeRate`] (sat/kwu, ceiling).
    ///
    /// 1 vB = 4 weight units; `sat/kvB / 4 = sat/kwu` (ceiling so we never underpay).
    pub fn to_bdk(self) -> bdk_wallet::bitcoin::FeeRate {
        let sat_per_kwu = self.0.div_ceil(4);
        bdk_wallet::bitcoin::FeeRate::from_sat_per_kwu(sat_per_kwu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Construction / validation
    // -------------------------------------------------------------------------

    #[test]
    fn new_happy_path_at_min_relay() {
        let r = FeeRate::new(1_000, 1_000);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().sat_per_kvb(), 1_000);
    }

    #[test]
    fn new_rejects_zero() {
        assert_eq!(FeeRate::new(0, 1_000), Err(FeeRateError::Zero));
    }

    #[test]
    fn new_rejects_below_min_relay() {
        let err = FeeRate::new(999, 1_000).unwrap_err();
        assert_eq!(
            err,
            FeeRateError::BelowMinRelay {
                given: 999,
                min_relay: 1_000
            }
        );
    }

    #[test]
    fn new_accepts_exactly_max() {
        assert!(FeeRate::new(MAX_FEE_RATE_SAT_PER_KVB, 1_000).is_ok());
    }

    #[test]
    fn new_rejects_above_max() {
        let err = FeeRate::new(MAX_FEE_RATE_SAT_PER_KVB + 1, 1_000).unwrap_err();
        assert_eq!(
            err,
            FeeRateError::AboveMax {
                given: MAX_FEE_RATE_SAT_PER_KVB + 1,
                max: MAX_FEE_RATE_SAT_PER_KVB
            }
        );
    }

    // -------------------------------------------------------------------------
    // fee_sats — golden vectors from the spec
    // -------------------------------------------------------------------------

    #[test]
    fn fee_sats_exact_1_sat_per_vb_350_vbytes() {
        // 1_000 sat/kvB × 350 vB = 350 sats (exact, no ceil needed)
        let r = FeeRate::new(1_000, 1_000).unwrap();
        assert_eq!(r.fee_sats(350), 350);
    }

    #[test]
    fn fee_sats_1100_sat_per_kvb_350_vbytes() {
        // 1_100 × 350 / 1000 = 385.0 → 385
        let r = FeeRate::new(1_100, 1_000).unwrap();
        assert_eq!(r.fee_sats(350), 385);
    }

    #[test]
    fn fee_sats_ceil_1001_sat_per_kvb_350_vbytes() {
        // 1_001 × 350 / 1000 = 350.35 → ceil → 351
        let r = FeeRate::new(1_001, 1_000).unwrap();
        assert_eq!(r.fee_sats(350), 351);
    }

    #[test]
    fn fee_sats_01_sat_per_vb_step_350_vbytes() {
        // 100 sat/kvB × 350 / 1000 = 35.0 → 35
        let r = FeeRate::new(100, 100).unwrap();
        assert_eq!(r.fee_sats(350), 35);
    }

    // -------------------------------------------------------------------------
    // with_margin_pct — golden vectors
    // -------------------------------------------------------------------------

    #[test]
    fn margin_20pct() {
        // 1_000 + 20% = 1_200 (exact)
        let r = FeeRate::new(1_000, 1_000).unwrap();
        assert_eq!(r.with_margin_pct(20).sat_per_kvb(), 1_200);
    }

    #[test]
    fn margin_10pct() {
        let r = FeeRate::new(1_000, 1_000).unwrap();
        assert_eq!(r.with_margin_pct(10).sat_per_kvb(), 1_100);
    }

    #[test]
    fn margin_5pct() {
        // 1_000 * 105 / 100 = 1_050 (exact)
        let r = FeeRate::new(1_000, 1_000).unwrap();
        assert_eq!(r.with_margin_pct(5).sat_per_kvb(), 1_050);
    }

    #[test]
    fn margin_saturates_at_max() {
        // A very large rate +20% would overflow without saturation
        let r = FeeRate::new(MAX_FEE_RATE_SAT_PER_KVB, 1_000).unwrap();
        assert_eq!(
            r.with_margin_pct(20).sat_per_kvb(),
            MAX_FEE_RATE_SAT_PER_KVB
        );
    }

    // -------------------------------------------------------------------------
    // round_up_to_step — golden vectors
    // -------------------------------------------------------------------------

    #[test]
    fn round_up_exact_step_unchanged() {
        // 1_100 is already a multiple of 100 → stays 1_100
        let r = FeeRate::new(1_100, 1_000).unwrap();
        assert_eq!(r.round_up_to_step().sat_per_kvb(), 1_100);
    }

    #[test]
    fn round_up_1050_to_1100() {
        // 1_050 → next step is 1_100 (step = 100)
        let r = FeeRate::new(1_050, 1_000).unwrap();
        assert_eq!(r.round_up_to_step().sat_per_kvb(), 1_100);
    }

    #[test]
    fn round_up_1001_to_1100() {
        // 1_001 → ceil to 1_100
        let r = FeeRate::new(1_001, 1_000).unwrap();
        assert_eq!(r.round_up_to_step().sat_per_kvb(), 1_100);
    }

    #[test]
    fn round_up_combined_spec_vector_5pct_margin_then_step() {
        // Spec: 1_000 +5% = 1_050 → round_up → 1_100
        let r = FeeRate::new(1_000, 1_000).unwrap().with_margin_pct(5);
        assert_eq!(r.sat_per_kvb(), 1_050);
        assert_eq!(r.round_up_to_step().sat_per_kvb(), 1_100);
    }

    // -------------------------------------------------------------------------
    // to_bdk — sat/kwu conversion (1 vB = 4 WU, ceiling)
    // -------------------------------------------------------------------------

    #[test]
    fn to_bdk_exact_multiple_of_4() {
        // 2_000 sat/kvB / 4 = 500 sat/kwu (exact)
        let r = FeeRate::new(2_000, 1_000).unwrap();
        assert_eq!(r.to_bdk().to_sat_per_kwu(), 500);
    }

    #[test]
    fn to_bdk_ceiling_2530_sat_per_kvb() {
        // Spec vector: 2_530 / 4 = 632.5 → ceil → 633
        let r = FeeRate::new(2_530, 1_000).unwrap();
        assert_eq!(r.to_bdk().to_sat_per_kwu(), 633);
    }

    #[test]
    fn to_bdk_never_underpays() {
        // For any valid rate, sat_per_kwu * 4 >= sat_per_kvb  (ceil guarantee)
        for sat_per_kvb in [
            1_000u64,
            1_001,
            1_100,
            2_530,
            9_999_999,
            MAX_FEE_RATE_SAT_PER_KVB,
        ] {
            let r = FeeRate::new(sat_per_kvb, 1_000).unwrap();
            let kwu = r.to_bdk().to_sat_per_kwu();
            assert!(
                kwu * 4 >= sat_per_kvb,
                "underpays: {sat_per_kvb} sat/kvB → {kwu} sat/kwu"
            );
        }
    }
}
