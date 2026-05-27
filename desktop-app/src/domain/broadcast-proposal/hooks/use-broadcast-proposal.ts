import { useEffect, useRef, useState } from 'react'
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

	useEffect(() => {
		if (!actionId) return
		let active = true
		setPhase('preparing')
		setError(null)
		Promise.all([
			prepareBroadcast(buildBroadcastInput(baseUrl, actionId)),
			getProposalByActionId({ baseUrl, actionId }),
		]).then(([res, proposalRes]) => {
			if (!active) return
			if (!res.ok) {
				setError(res.error)
				setPhase('error')
				return
			}
			if (proposalRes.ok) {
				const p = proposalRes.data
				setProposal(p)
				if (p.broadcastStatus !== 'idle' && p.broadcastStatus !== 'failed') {
					setResult(
						mergeBroadcastWithProposal(
							{
								actionId: p.actionId,
								proposalStatus: p.status,
								broadcastStatus: p.broadcastStatus,
								commitTxid: p.commitTxid,
								revealTxid: p.revealTxid,
							},
							p,
						),
					)
					setBundle(res.data)
					setPhase('done')
					return
				}
			}
			setBundle(res.data)
			setPhase('confirming')
		})
		return () => {
			active = false
		}
	}, [actionId, baseUrl])

	async function prepare() {
		setPhase('preparing')
		setError(null)
		const [res, proposalRes] = await Promise.all([
			prepareBroadcast(buildBroadcastInput(baseUrl, actionId)),
			getProposalByActionId({ baseUrl, actionId }),
		])
		if (!res.ok) {
			setError(res.error)
			setPhase('error')
			return
		}
		if (proposalRes.ok) {
			setProposal(proposalRes.data)
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
