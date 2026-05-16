import { useRef, useState } from 'react'
import {
	broadcastProposal,
	getProposalByActionId,
	prepareBroadcast,
	type BroadcastInput,
	type BroadcastResult,
	type PrepareBroadcastResult,
	type Proposal,
} from '@/api/proposals'

import type { BroadcastPhase } from '../model/broadcast-proposal'

const inFlightActionIds = new Set<string>()

type UseBroadcastProposalReturn = {
	phase: BroadcastPhase
	bundle: PrepareBroadcastResult | null
	result: BroadcastResult | null
	/** Authoritative proposal row from orchestrator after broadcast (P-062). */
	proposal: Proposal | null
	error: string | null
	prepare: () => Promise<void>
	broadcast: () => Promise<void>
}

/** Merge broadcast IPC result with a fresh orchestrator proposal row. */
export function mergeBroadcastWithProposal(broadcast: BroadcastResult, proposal: Proposal): BroadcastResult {
	return {
		actionId: proposal.actionId,
		proposalStatus: proposal.status,
		broadcastStatus: proposal.broadcastStatus,
		commitTxid: proposal.commitTxid ?? broadcast.commitTxid,
		revealTxid: proposal.revealTxid ?? broadcast.revealTxid,
	}
}

function buildBroadcastInput(baseUrl: string, actionId: string): BroadcastInput {
	return { baseUrl, actionId }
}

export function useBroadcastProposal(baseUrl: string, actionId: string): UseBroadcastProposalReturn {
	const [phase, setPhase] = useState<BroadcastPhase>('idle')
	const [bundle, setBundle] = useState<PrepareBroadcastResult | null>(null)
	const [result, setResult] = useState<BroadcastResult | null>(null)
	const [proposal, setProposal] = useState<Proposal | null>(null)
	const [error, setError] = useState<string | null>(null)
	const broadcastStarted = useRef(false)

	async function prepare() {
		setPhase('preparing')
		setError(null)
		const res = await prepareBroadcast(buildBroadcastInput(baseUrl, actionId))
		if (!res.ok) {
			setError(res.error)
			setPhase('error')
			return
		}
		setBundle(res.data)
		setPhase('confirming')
	}

	async function broadcast() {
		if (broadcastStarted.current || inFlightActionIds.has(actionId)) {
			return
		}
		broadcastStarted.current = true
		inFlightActionIds.add(actionId)
		setPhase('broadcasting')
		setError(null)
		try {
			const res = await broadcastProposal(buildBroadcastInput(baseUrl, actionId))
			if (!res.ok) {
				setError(res.error)
				setPhase('error')
				return
			}

			const refreshed = await getProposalByActionId({ baseUrl, actionId })
			if (refreshed.ok) {
				setProposal(refreshed.data)
				setResult(mergeBroadcastWithProposal(res.data, refreshed.data))
			} else {
				setResult(res.data)
			}
			setPhase('done')
		} finally {
			inFlightActionIds.delete(actionId)
		}
	}

	return { phase, bundle, result, proposal, error, prepare, broadcast }
}
