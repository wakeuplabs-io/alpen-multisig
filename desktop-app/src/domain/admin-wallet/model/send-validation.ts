// Phase 6 (PRD §4.3.5) — pure helpers for the Send form. The backend is the
// validation authority (address parse, coin selection); these helpers only
// provide inline, pre-submit feedback and the Confirm gate.

/**
 * Parses a sats amount input: digits only, returns null for empty/invalid.
 * Rejects signs, decimals, exponents, and non-safe integers.
 */
export function parseAmountSats(raw: string): number | null {
	const trimmed = raw.trim()
	if (!/^\d+$/.test(trimmed)) return null
	const val = Number(trimmed)
	if (!Number.isSafeInteger(val)) return null
	return val
}

export type SendConfirmGate = {
	hasDestination: boolean
	amountSats: number | null
	isFeeReady: boolean
	isSubmitting: boolean
}

/**
 * P6.1 interim Confirm gate (PRD §4.3.5.5): destination entered, positive
 * amount, fee presets loaded, not mid-submit. P6.2/P6.3 tighten this with
 * backend-validated destination and estimate readiness; the backend rejects
 * independently either way.
 */
export function canConfirmSend(gate: SendConfirmGate): boolean {
	return gate.hasDestination && gate.amountSats !== null && gate.amountSats > 0 && gate.isFeeReady && !gate.isSubmitting
}
