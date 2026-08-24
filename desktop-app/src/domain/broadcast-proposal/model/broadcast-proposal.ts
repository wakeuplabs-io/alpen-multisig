import type { BroadcastStatus } from '@/api/proposals'

export type BroadcastPhase =
	'idle' | 'preparing' | 'confirming' | 'awaiting-device' | 'broadcasting' | 'awaiting-confirmation' | 'done' | 'error'

/**
 * Resolve the post-submit phase from the authoritative broadcast/proposal status.
 *
 * - `reveal_confirmed` (or an already-`enacted` proposal) → `done`.
 * - `reveal_broadcasted` / `commit_broadcasted` / `commit_confirmed` → `awaiting-confirmation`
 *   (submitted, the reveal is in the mempool awaiting a block — the user may leave).
 * - anything else (`idle`, `failed`) → `null`, leaving the caller's current phase unchanged.
 */
export function phaseForBroadcastStatus(
	broadcastStatus: BroadcastStatus,
	proposalStatus?: string,
): Extract<BroadcastPhase, 'done' | 'awaiting-confirmation'> | null {
	if (broadcastStatus === 'reveal_confirmed' || proposalStatus === 'enacted') return 'done'
	if (
		broadcastStatus === 'reveal_broadcasted' ||
		broadcastStatus === 'commit_broadcasted' ||
		broadcastStatus === 'commit_confirmed'
	) {
		return 'awaiting-confirmation'
	}
	return null
}

export type BroadcastErrorCode =
	| 'insufficient_fee'
	| 'mempool_rejected'
	| 'double_spend'
	| 'consensus_violation'
	| 'invalid_reveal'
	| 'orphan_commit'
	| 'device_disconnected'
	| 'hw_signing_failed'
	| 'session_expired'
	| 'broadcast_unavailable'
	| 'unknown_error'

export type BroadcastRecovery = 'retry' | 'resubmit-reveal' | 'reconnect-device' | 're-auth' | 'manual-broadcast'

export type BroadcastError = {
	code: BroadcastErrorCode
	message: string
	recovery: BroadcastRecovery
	commitTxHex?: string
	revealTxHex?: string
}

const CODE_RECOVERY_MAP: Record<string, BroadcastRecovery> = {
	insufficient_fee: 'retry',
	mempool_rejected: 'retry',
	double_spend: 'retry',
	consensus_violation: 'resubmit-reveal',
	invalid_reveal: 'resubmit-reveal',
	orphan_commit: 'resubmit-reveal',
	device_disconnected: 'reconnect-device',
	hw_signing_failed: 'reconnect-device',
	session_expired: 're-auth',
	broadcast_unavailable: 'manual-broadcast',
	unknown_error: 'retry',
}

export function deriveBroadcastError(raw: string): BroadcastError {
	try {
		const parsed = JSON.parse(raw) as {
			code?: string
			message?: string
			commitTxHex?: string
			revealTxHex?: string
		}
		const code = (parsed.code && CODE_RECOVERY_MAP[parsed.code] ? parsed.code : 'unknown_error') as BroadcastErrorCode
		return {
			code,
			message: parsed.message ?? raw,
			recovery: CODE_RECOVERY_MAP[code],
			commitTxHex: parsed.commitTxHex,
			revealTxHex: parsed.revealTxHex,
		}
	} catch {
		return makeBroadcastError('unknown_error', raw, 'retry')
	}
}

function makeBroadcastError(code: BroadcastErrorCode, message: string, recovery: BroadcastRecovery): BroadcastError {
	return { code, message, recovery }
}

export function satsToBtc(sats: number): string {
	return (sats / 1e8).toFixed(8)
}

export function isTerminal(status: BroadcastStatus): boolean {
	return status === 'reveal_confirmed' || status === 'failed'
}

// ── Phase-driven view predicates ─────────────────────────────────────────────
// Shared by the send-proposal and send-cancel screens: both render the same
// stepper/card/progress stack, and keeping the predicates here stops the two
// screens from drifting apart (the cancel screen used to omit `awaiting-device`
// and `awaiting-confirmation`, which blanked the UI while the reveal confirmed).

/** Skeleton while the commit bundle is being prepared. */
export function isBroadcastLoadingPhase(phase: BroadcastPhase): boolean {
	return phase === 'idle' || phase === 'preparing'
}

/** Details card is visible — callers must additionally require a non-null bundle. */
export function isBroadcastDetailsPhase(phase: BroadcastPhase): boolean {
	return phase === 'confirming' || phase === 'awaiting-device' || phase === 'broadcasting'
}

/**
 * Phase-progress panel is visible. Overlaps `isBroadcastDetailsPhase` on
 * `awaiting-device` by design: the card shows the device prompt while the panel
 * shows the commit/reveal step.
 */
export function isBroadcastProgressPhase(phase: BroadcastPhase): boolean {
	return (
		phase === 'awaiting-device' ||
		phase === 'broadcasting' ||
		phase === 'awaiting-confirmation' ||
		phase === 'done' ||
		phase === 'error'
	)
}

/** A submit is in flight — drives the details card's `isBroadcasting`. */
export function isBroadcastInFlightPhase(phase: BroadcastPhase): boolean {
	return phase === 'broadcasting' || phase === 'awaiting-device'
}

export type BroadcastConfirmGateInput = {
	isBroadcasting: boolean
	canSign: boolean
	/** Cancel flow only: `false` means the targeted action is no longer queued on the ASM. */
	targetQueued: boolean | null | undefined
	/** `null` = still loading, `undefined` = unavailable — neither can fund a commit. */
	adminWalletInfo: { balanceSats: number } | null | undefined
}

/**
 * Whether "Confirm & Send" must stay disabled.
 *
 * `adminWalletInfo == null` is loose on purpose — a wallet that is still loading and one that is
 * unavailable are equally unable to pay the commit. `targetQueued === false` is strict on purpose:
 * `null`/`undefined` mean "not applicable / not checked yet" and must not block the send.
 */
export function isBroadcastConfirmDisabled(input: BroadcastConfirmGateInput): boolean {
	return (
		input.isBroadcasting ||
		!input.canSign ||
		input.targetQueued === false ||
		input.adminWalletInfo == null ||
		input.adminWalletInfo.balanceSats === 0
	)
}
