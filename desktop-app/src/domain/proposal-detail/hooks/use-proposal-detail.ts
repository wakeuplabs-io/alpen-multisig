import { useCallback, useEffect, useState } from 'react'
import { getProposalByActionId, type Proposal } from '@/api/proposals'

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

	useEffect(() => {
		let cancelled = false
		setIsLoading(true)
		setError(null)
		void getProposalByActionId({ baseUrl, actionId }).then((res) => {
			if (cancelled) return
			if (res.ok) {
				setProposal(res.data)
			} else {
				setError(res.error)
			}
			setIsLoading(false)
		})
		return () => {
			cancelled = true
		}
	}, [baseUrl, actionId, tick])

	// A failed background read keeps the proposal on screen: the next one tries again.
	const refresh = useCallback(() => {
		void getProposalByActionId({ baseUrl, actionId }).then((res) => {
			if (res.ok) setProposal(res.data)
		})
	}, [baseUrl, actionId])

	return { proposal, isLoading, error, reload: () => setTick((t) => t + 1), refresh }
}
