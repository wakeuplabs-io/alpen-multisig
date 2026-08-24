import { FEE_RATE_STEP_SAT_PER_KVB } from '@/domain/fee-selection/model/fee-rate'

/** 1 sat/vB — static floor when live fee presets are unavailable (mirrors backend fallback). */
export const FALLBACK_MIN_RELAY_SAT_PER_KVB = 1_000

/** 10,000 sat/vB — PRD hard ceiling, used when live presets are unavailable. */
export const FALLBACK_MAX_SAT_PER_KVB = 10_000_000

/**
 * Lowest acceptable replacement rate: one UI step (0.1 sat/vB) above the current
 * rate — BIP-125 requires the replacement to pay strictly more. Falls back to the
 * minimum relay rate when the current rate is unknown.
 */
export function minBumpRateSatPerKvb(currentSatPerKvb: number | null, minRelaySatPerKvb: number): number {
	if (currentSatPerKvb === null) return minRelaySatPerKvb
	return currentSatPerKvb + FEE_RATE_STEP_SAT_PER_KVB
}

/** Suggested default for the bump input: the Fast preset, raised to at least the minimum bump rate. */
export function suggestedBumpRateSatPerKvb(fastPresetSatPerKvb: number | null, minBumpSatPerKvb: number): number {
	if (fastPresetSatPerKvb === null) return minBumpSatPerKvb
	return Math.max(fastPresetSatPerKvb, minBumpSatPerKvb)
}

/**
 * The ceiling that actually applies to a row: the general one, lowered by whatever the row
 * itself can carry.
 *
 * CPFP rows come with their own limit — the child pays the whole package's shortfall out of
 * its own vsize, so its individual rate runs several times the requested package rate and
 * hits the per-transaction broadcast ceiling long before the package does (#431). RBF rows
 * send `null` and keep the general ceiling.
 */
export function effectiveMaxBumpRate(generalMaxSatPerKvb: number, rowMaxSatPerKvb: number | null): number {
	if (rowMaxSatPerKvb === null) return generalMaxSatPerKvb
	return Math.min(generalMaxSatPerKvb, rowMaxSatPerKvb)
}

/** A candidate replacement rate is valid when within [minBump, max]. */
export function isValidBumpRate(satPerKvb: number | null, minBumpSatPerKvb: number, maxSatPerKvb: number): boolean {
	return satPerKvb !== null && satPerKvb >= minBumpSatPerKvb && satPerKvb <= maxSatPerKvb
}
