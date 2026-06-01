import type { BroadcastStatus } from '@/api/proposals'

export type BroadcastPhase = 'idle' | 'preparing' | 'confirming' | 'awaiting-device' | 'broadcasting' | 'done' | 'error'

export type BroadcastErrorCode =
	| 'insufficient_fee'
	| 'mempool_rejected'
	| 'double_spend'
	| 'consensus_violation'
	| 'invalid_reveal'
	| 'orphan_commit'
	| 'device_disconnected'
	| 'session_expired'
	| 'unknown_error'
	| 'Unknown'

export type BroadcastRecovery = 'retry' | 'resubmit-reveal' | 'reconnect-device' | 're-auth'

export type BroadcastError = {
	code: BroadcastErrorCode
	message: string
	recovery: BroadcastRecovery
}

const CODE_RECOVERY_MAP: Record<string, BroadcastRecovery> = {
	insufficient_fee: 'retry',
	mempool_rejected: 'retry',
	double_spend: 'retry',
	consensus_violation: 'resubmit-reveal',
	invalid_reveal: 'resubmit-reveal',
	orphan_commit: 'resubmit-reveal',
	device_disconnected: 'reconnect-device',
	session_expired: 're-auth',
	unknown_error: 'retry',
}

export function deriveBroadcastError(raw: string): BroadcastError {
	try {
		const parsed = JSON.parse(raw) as { code?: string; message?: string }
		const code = (parsed.code && CODE_RECOVERY_MAP[parsed.code] ? parsed.code : 'unknown_error') as BroadcastErrorCode
		return {
			code,
			message: parsed.message ?? raw,
			recovery: CODE_RECOVERY_MAP[code],
		}
	} catch {
		return {
			code: 'Unknown',
			message: raw,
			recovery: 'retry',
		}
	}
}

export function satsToBtc(sats: number): string {
	return (sats / 1e8).toFixed(8)
}

export function isTerminal(status: BroadcastStatus): boolean {
	return status === 'reveal_confirmed' || status === 'failed'
}
