import { useEffect, useState } from 'react'
import { getProposalByActionId, type Proposal } from '@/api/proposals'

type UseCancelProposalDetailsReturn = {
	/** The cancel proposal itself — carries its own seqNo and actionHex. */
	cancelProposal: Proposal | null
	isLoading: boolean
	error: string | null
}

/**
 * Loads the cancel proposal by its own action id.
 *
 * `Proposal.cancelProposal` is a thin summary (signatures + quorum only), so the cancellation's
 * sequence number and reviewable payload have to be read from the proposal row itself — the same
 * fetch the cancel sign screen already does before prompting the device.
 */
export function useCancelProposalDetails(
	baseUrl: string,
	cancelActionId: string | null,
): UseCancelProposalDetailsReturn {
	const [cancelProposal, setCancelProposal] = useState<Proposal | null>(null)
	const [isLoading, setIsLoading] = useState(false)
	const [error, setError] = useState<string | null>(null)

	useEffect(() => {
		if (cancelActionId === null) {
			setCancelProposal(null)
			setIsLoading(false)
			setError(null)
			return
		}

		let cancelled = false
		setIsLoading(true)
		setError(null)
		void getProposalByActionId({ baseUrl, actionId: cancelActionId }).then((res) => {
			if (cancelled) return
			if (res.ok) {
				setCancelProposal(res.data)
			} else {
				setCancelProposal(null)
				setError(res.error)
			}
			setIsLoading(false)
		})

		return () => {
			cancelled = true
		}
	}, [baseUrl, cancelActionId])

	return { cancelProposal, isLoading, error }
}
