import { useBroadcastProposal } from '@/domain/broadcast-proposal/hooks/use-broadcast-proposal'
import { useProposalDetail } from '@/domain/proposal-detail/hooks/use-proposal-detail'
import type { BroadcastError, BroadcastPhase } from '@/domain/broadcast-proposal/model/broadcast-proposal'
import { deriveBroadcastError } from '@/domain/broadcast-proposal/model/broadcast-proposal'
import type { BroadcastResult, PrepareBroadcastResult, Proposal } from '@/api/proposals'

type UseCancelBroadcastReturn = {
	isResolvingCancel: boolean
	cancelResolveError: BroadcastError | null
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

export function useCancelBroadcast(
	baseUrl: string,
	targetActionId: string,
	/** `null` while fee presets load — broadcast stays blocked until ready. */
	feeRateSatPerKvb: number | null,
): UseCancelBroadcastReturn {
	const { proposal: targetProposal, isLoading, error: targetError } = useProposalDetail(baseUrl, targetActionId)

	const cancelActionId = targetProposal?.cancelProposal?.actionId ?? null

	const broadcastState = useBroadcastProposal(baseUrl, cancelActionId ?? '', feeRateSatPerKvb)

	return {
		isResolvingCancel: isLoading,
		cancelResolveError: targetError != null ? deriveBroadcastError(targetError) : null,
		cancelActionId,
		...broadcastState,
	}
}
