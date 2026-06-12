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
	/** True only when the backend validated the destination (P6.2, §4.3.5.1). */
	isDestinationValid: boolean
	amountSats: number | null
	/** Max mode (drain) — the amount field tracks the boundary, not the user. */
	isMax: boolean
	isFeeReady: boolean
	/** True only when the dry-run estimate succeeded (P6.3, §4.3.5.2). */
	isEstimateReady: boolean
	isSubmitting: boolean
}

/**
 * Confirm gate (PRD §4.3.5.5): backend-validated destination, positive amount
 * (or Max/drain mode), fee presets loaded, dry-run estimate succeeded — an
 * insufficient-funds or below-dust estimate keeps Confirm disabled — and not
 * mid-submit. The backend rejects independently either way.
 */
export function canConfirmSend(gate: SendConfirmGate): boolean {
	const hasAmount = gate.isMax || (gate.amountSats !== null && gate.amountSats > 0)
	return gate.isDestinationValid && hasAmount && gate.isFeeReady && gate.isEstimateReady && !gate.isSubmitting
}
