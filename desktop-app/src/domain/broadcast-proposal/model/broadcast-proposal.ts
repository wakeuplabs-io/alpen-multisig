import type { BroadcastStatus } from '@/api/proposals'

export type BroadcastPhase = 'idle' | 'preparing' | 'confirming' | 'broadcasting' | 'done' | 'error'

export function satsToBtc(sats: number): string {
	return (sats / 1e8).toFixed(8)
}

export function isTerminal(status: BroadcastStatus): boolean {
	return status === 'reveal_confirmed' || status === 'failed'
}
