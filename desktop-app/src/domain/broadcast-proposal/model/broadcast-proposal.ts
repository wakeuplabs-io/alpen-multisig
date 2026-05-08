import type { BroadcastStatus } from '@/api/proposals'

export type BroadcastPhase = 'idle' | 'preparing' | 'confirming' | 'broadcasting' | 'done' | 'error'

export function broadcastStatusToPhase(status: BroadcastStatus): BroadcastPhase {
	switch (status) {
		case 'idle':
			return 'idle'
		case 'commit_broadcasted':
			return 'broadcasting'
		case 'commit_confirmed':
			return 'broadcasting'
		case 'reveal_broadcasted':
			return 'broadcasting'
		case 'reveal_confirmed':
			return 'done'
		case 'failed':
			return 'error'
	}
}

export function satsToBtc(sats: number): string {
	return (sats / 1e8).toFixed(8)
}

export function broadcastStatusLabel(status: BroadcastStatus): string {
	switch (status) {
		case 'idle':
			return 'Not started'
		case 'commit_broadcasted':
			return 'Commit broadcast — awaiting confirmation'
		case 'commit_confirmed':
			return 'Commit confirmed — broadcasting reveal'
		case 'reveal_broadcasted':
			return 'Reveal broadcast — awaiting confirmation'
		case 'reveal_confirmed':
			return 'Enacted'
		case 'failed':
			return 'Failed'
	}
}

export function isTerminal(status: BroadcastStatus): boolean {
	return status === 'reveal_confirmed' || status === 'failed'
}
