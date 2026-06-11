import type { BroadcastStatus } from '@/api/proposals'

export type BroadcastPhase =
	| 'idle'
	| 'preparing'
	| 'confirming'
	| 'awaiting-device'
	| 'broadcasting'
	| 'awaiting-confirmation'
	| 'done'
	| 'error'

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
