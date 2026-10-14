/**
 * Orders the reads that write one piece of proposal state: the visible load and the silent refresh.
 *
 * Both write the same state, and a response can arrive after a later one has. Each read takes a
 * ticket, and only the latest ticket may write. A refresh never starts while a load is in flight —
 * the load is about to answer with fresh data anyway — so a refresh can never be the read that
 * leaves a load's spinner running.
 */
export type ProposalReadGate = {
	beginLoad: () => number
	/** `null` while a load is in flight: skip this refresh. */
	beginRefresh: () => number | null
	isLatest: (ticket: number) => boolean
	settle: (ticket: number) => void
}

export function createProposalReadGate(): ProposalReadGate {
	let latest = 0
	let loadInFlight: number | null = null
	return {
		beginLoad() {
			latest += 1
			loadInFlight = latest
			return latest
		},
		beginRefresh() {
			if (loadInFlight !== null) return null
			latest += 1
			return latest
		},
		isLatest(ticket) {
			return ticket === latest
		},
		settle(ticket) {
			if (loadInFlight === ticket) loadInFlight = null
		},
	}
}
