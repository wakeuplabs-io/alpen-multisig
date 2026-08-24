import { useEffect, useState } from 'react'
import { useBroadcastProposal } from '@/domain/broadcast-proposal/hooks/use-broadcast-proposal'
import type { SignerKind } from '@/domain/broadcast-proposal/hooks/use-broadcast-proposal'
import { useProposalDetail } from '@/domain/proposal-detail/hooks/use-proposal-detail'
import type { BroadcastError, BroadcastPhase } from '@/domain/broadcast-proposal/model/broadcast-proposal'
import { deriveBroadcastError } from '@/domain/broadcast-proposal/model/broadcast-proposal'
import {
	checkCancelTargetQueued,
	type BroadcastResult,
	type PrepareBroadcastResult,
	type Proposal,
} from '@/api/proposals'
import type { WalletAdapter } from '@/wallet/types'

type UseCancelBroadcastReturn = {
	isResolvingCancel: boolean
	cancelResolveError: BroadcastError | null
	cancelActionId: string | null
	isCheckingTargetQueued: boolean
	targetQueued: boolean | null
	targetQueuedError: string | null
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
	/** Drives the `awaiting-device` phase — a hardware signer must be prompted before the commit. */
	signerKind: SignerKind = 'mnemonic',
	/** Required to (re)bind the Admin Wallet session and check the hardware binding before sending. */
	adapter?: WalletAdapter,
): UseCancelBroadcastReturn {
	const { proposal: targetProposal, isLoading, error: targetError } = useProposalDetail(baseUrl, targetActionId)

	const cancelActionId = targetProposal?.cancelProposal?.actionId ?? null

	const broadcastState = useBroadcastProposal(baseUrl, cancelActionId ?? '', feeRateSatPerKvb, signerKind, adapter)

	const [isCheckingTargetQueued, setIsCheckingTargetQueued] = useState(false)
	const [targetQueued, setTargetQueued] = useState<boolean | null>(null)
	const [targetQueuedError, setTargetQueuedError] = useState<string | null>(null)

	useEffect(() => {
		if (cancelActionId == null) {
			setTargetQueued(null)
			setTargetQueuedError(null)
			return
		}
		let cancelled = false
		setIsCheckingTargetQueued(true)
		setTargetQueuedError(null)
		void checkCancelTargetQueued({ baseUrl, actionId: cancelActionId }).then((res) => {
			if (cancelled) return
			if (res.ok) {
				setTargetQueued(res.data)
			} else {
				setTargetQueued(null)
				setTargetQueuedError(res.error)
			}
			setIsCheckingTargetQueued(false)
		})
		return () => {
			cancelled = true
		}
	}, [baseUrl, cancelActionId])

	return {
		isResolvingCancel: isLoading,
		cancelResolveError: targetError != null ? deriveBroadcastError(targetError) : null,
		cancelActionId,
		isCheckingTargetQueued,
		targetQueued,
		targetQueuedError,
		...broadcastState,
	}
}
