import { useCallback, useEffect, useState } from 'react'
import { getProposalByActionId, type Proposal } from '@/api/proposals'
import { createProposalReadGate } from '@/lib/proposal-read-gate'

type UseProposalDetailReturn = {
	proposal: Proposal | null
	isLoading: boolean
	error: string | null
	reload: () => void
	/** Fetches again without toggling `isLoading`, so the screen keeps showing what it has. */
	refresh: () => void
}

export function useProposalDetail(baseUrl: string, actionId: string): UseProposalDetailReturn {
	const [proposal, setProposal] = useState<Proposal | null>(null)
	const [isLoading, setIsLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)
	const [tick, setTick] = useState(0)
	// The load and the refresh write the same state: a slower, older response must not land.
	const [gate] = useState(createProposalReadGate)

	useEffect(() => {
		const ticket = gate.beginLoad()
		setIsLoading(true)
		setError(null)
		void getProposalByActionId({ baseUrl, actionId }).then((res) => {
			gate.settle(ticket)
			if (!gate.isLatest(ticket)) return
			if (res.ok) {
				setProposal(res.data)
			} else {
				setError(res.error)
			}
			setIsLoading(false)
		})
	}, [gate, baseUrl, actionId, tick])

	// A failed background read keeps the proposal on screen: the next one tries again.
	const refresh = useCallback(() => {
		const ticket = gate.beginRefresh()
		if (ticket === null) return
		void getProposalByActionId({ baseUrl, actionId }).then((res) => {
			if (gate.isLatest(ticket) && res.ok) setProposal(res.data)
		})
	}, [gate, baseUrl, actionId])

	return { proposal, isLoading, error, reload: () => setTick((t) => t + 1), refresh }
}
