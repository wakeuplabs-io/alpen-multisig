import { useBroadcastProposal } from '@/domain/broadcast-proposal/hooks/use-broadcast-proposal'
import { useProposalDetail } from '@/domain/proposal-detail/hooks/use-proposal-detail'
import type { BroadcastError, BroadcastPhase } from '@/domain/broadcast-proposal/model/broadcast-proposal'
import type { BroadcastResult, PrepareBroadcastResult, Proposal } from '@/api/proposals'

type UseCancelBroadcastReturn = {
	isResolvingCancel: boolean
	cancelResolveError: string | null
	cancelActionId: string | null
	phase: BroadcastPhase
	bundle: PrepareBroadcastResult | null
	result: BroadcastResult | null
	proposal: Proposal | null
	error: BroadcastError | null
	canResubmit: boolean
	prepare: () => Promise<void>
	broadcast: () => Promise<void>
}

export function useCancelBroadcast(baseUrl: string, targetActionId: string): UseCancelBroadcastReturn {
	const { proposal: targetProposal, isLoading, error: targetError } = useProposalDetail(baseUrl, targetActionId)

	const cancelActionId = targetProposal?.cancelProposal?.actionId ?? ''

	const broadcastState = useBroadcastProposal(baseUrl, cancelActionId)

	return {
		isResolvingCancel: isLoading,
		cancelResolveError: targetError,
		cancelActionId: cancelActionId || null,
		...broadcastState,
	}
}
