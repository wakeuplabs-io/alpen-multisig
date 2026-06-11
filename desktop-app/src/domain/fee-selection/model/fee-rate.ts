import type { FeeRates } from '@/api/fee-rates'

export const FEE_RATE_STEP_SAT_PER_KVB = 100 // 0.1 sat/vB
export const SAT_PER_VB_TO_KVB = 1_000

export type FeePresetKey = 'slow' | 'medium' | 'fast'

export type FeeSelection = { kind: 'preset'; preset: FeePresetKey } | { kind: 'custom'; satPerKvb: number }

/** Fee in sats for `vbytes` virtual bytes, rounded up. */
export function feeSats(satPerKvb: number, vbytes: number): number {
	return Math.ceil((satPerKvb * vbytes) / SAT_PER_VB_TO_KVB)
}

/** Display string: convert sat/kvB integer to sat/vB decimal with one decimal place. */
export function formatSatPerVb(satPerKvb: number): string {
	return (satPerKvb / SAT_PER_VB_TO_KVB).toFixed(1)
}

/** Convert sat/vB string input to sat/kvB integer, or null if invalid. */
export function parseSatPerVb(input: string): number | null {
	const val = parseFloat(input)
	if (!isFinite(val) || val <= 0) return null
	return Math.round(val * SAT_PER_VB_TO_KVB)
}

/** Resolve the sat/kvB rate for a given selection against the loaded presets. */
export function resolveRate(selection: FeeSelection, presets: FeeRates): number {
	if (selection.kind === 'custom') return selection.satPerKvb
	return presets[selection.preset].satPerKvb
}

/** Clamp a custom sat/kvB value to [minRelay, max]. */
export function clampCustomRate(satPerKvb: number, presets: FeeRates): number {
	return Math.max(presets.minRelaySatPerKvb, Math.min(presets.maxSatPerKvb, satPerKvb))
}

/** Total fee sats for both commit and reveal transactions. */
export function totalFeeSats(satPerKvb: number, presets: FeeRates): number {
	const revealFee = feeSats(satPerKvb, presets.revealVbytes)
	const commitFee = feeSats(satPerKvb, presets.commitVbytesEstimate)
	return revealFee + commitFee
}
