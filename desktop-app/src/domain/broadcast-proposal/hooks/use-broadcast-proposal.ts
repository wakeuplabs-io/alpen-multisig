import { useCallback, useEffect, useRef, useState } from 'react'
import {
	broadcastProposal,
	getProposalByActionId,
	prepareBroadcast,
	type BroadcastInput,
	type BroadcastResult,
	type PrepareBroadcastResult,
	type Proposal,
} from '@/api/proposals'

import { getAdminWalletCanSign, walletSessionInit, walletSessionInitWatchOnly } from '@/api/admin-wallet'
import { adminWalletCapabilitySchema } from '@/api/ipc-schemas'
import { initAdminWalletForAdapter } from '@/contexts/session-provider-vendor-branch'
import type { WalletAdapter } from '@/wallet/types'

import type { BroadcastError, BroadcastPhase } from '../model/broadcast-proposal'
import { deriveBroadcastError, phaseForBroadcastStatus } from '../model/broadcast-proposal'

export type SignerKind = 'hardware' | 'mnemonic'

const inFlightActionIds = new Set<string>()

/** Interval between confirmation polls while in `awaiting-confirmation`. */
const CONFIRMATION_POLL_INTERVAL_MS = 8000

type UseBroadcastProposalReturn = {
	phase: BroadcastPhase
	bundle: PrepareBroadcastResult | null
	result: BroadcastResult | null
	/** Authoritative proposal row from orchestrator after broadcast (P-062). */
	proposal: Proposal | null
	error: BroadcastError | null
	/** True ONLY when error.recovery === 'resubmit-reveal'; false for all other errors. */
	canResubmit: boolean
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

export function useBroadcastProposal(
	baseUrl: string,
	actionId: string,
	signerKind: SignerKind = 'mnemonic',
	adapter?: WalletAdapter,
): UseBroadcastProposalReturn {
	const [phase, setPhase] = useState<BroadcastPhase>('idle')
	const [bundle, setBundle] = useState<PrepareBroadcastResult | null>(null)
	const [result, setResult] = useState<BroadcastResult | null>(null)
	const [proposal, setProposal] = useState<Proposal | null>(null)
	const [error, setError] = useState<BroadcastError | null>(null)
	const broadcastStarted = useRef(false)
	const pollRef = useRef<ReturnType<typeof setInterval> | null>(null)

	const clearConfirmationPoll = useCallback(() => {
		if (pollRef.current !== null) {
			clearInterval(pollRef.current)
			pollRef.current = null
		}
	}, [])

	/** Adopt a fresh orchestrator row into proposal/result state. */
	const applyProposal = useCallback((p: Proposal) => {
		setProposal(p)
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
	}, [])

	/** Poll the orchestrator until the reveal confirms, then flip to `done`. */
	const startConfirmationPoll = useCallback(() => {
		clearConfirmationPoll()
		pollRef.current = setInterval(() => {
			void getProposalByActionId({ baseUrl, actionId }).then((res) => {
				if (!res.ok) return
				applyProposal(res.data)
				if (phaseForBroadcastStatus(res.data.broadcastStatus, res.data.status) === 'done') {
					clearConfirmationPoll()
					setPhase('done')
				}
			})
		}, CONFIRMATION_POLL_INTERVAL_MS)
	}, [baseUrl, actionId, applyProposal, clearConfirmationPoll])

	// Stop any active poll when the hook unmounts.
	useEffect(() => clearConfirmationPoll, [clearConfirmationPoll])

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
				setError(deriveBroadcastError(res.error))
				setPhase('error')
				return
			}
			if (proposalRes.ok) {
				const p = proposalRes.data
				setProposal(p)
				const resumePhase = phaseForBroadcastStatus(p.broadcastStatus, p.status)
				if (resumePhase !== null) {
					applyProposal(p)
					setBundle(res.data)
					setPhase(resumePhase)
					// Reconcile: a still-unconfirmed reveal keeps polling until it confirms.
					if (resumePhase === 'awaiting-confirmation') startConfirmationPoll()
					return
				}
			}
			setBundle(res.data)
			setPhase('confirming')
		})
		return () => {
			active = false
		}
	}, [actionId, baseUrl, applyProposal, startConfirmationPoll])

	async function prepare() {
		setPhase('preparing')
		setError(null)
		const [res, proposalRes] = await Promise.all([
			prepareBroadcast(buildBroadcastInput(baseUrl, actionId)),
			getProposalByActionId({ baseUrl, actionId }),
		])
		if (!res.ok) {
			setError(deriveBroadcastError(res.error))
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
		if (adapter !== undefined) {
			const sessionInit = await initAdminWalletForAdapter(adapter, walletSessionInitWatchOnly, walletSessionInit)
			if (!sessionInit.ok) {
				setError(
					deriveBroadcastError(
						sessionInit.error ?? 'Admin Wallet session could not be started — disconnect and reconnect your wallet',
					),
				)
				setPhase('error')
				return
			}
			if (adapter.vendor === 'ledger' || adapter.vendor === 'trezor') {
				const capability = await getAdminWalletCanSign()
				const parsed = capability.ok ? adminWalletCapabilitySchema.safeParse(capability.data) : null
				if (parsed?.success && parsed.data.signerKind !== 'hardware') {
					setError(
						deriveBroadcastError(
							'Admin Wallet is not bound to your hardware device. Disconnect, connect with Ledger (not Palabras), authenticate, then try again.',
						),
					)
					setPhase('error')
					return
				}
			}
		}
		broadcastStarted.current = true
		inFlightActionIds.add(actionId)
		setError(null)
		if (signerKind === 'hardware') {
			setPhase('awaiting-device')
		} else {
			setPhase('broadcasting')
		}
		try {
			const res = await broadcastProposal(buildBroadcastInput(baseUrl, actionId))
			if (!res.ok) {
				setError(deriveBroadcastError(res.error))
				setPhase('error')
				return
			}

			// Submit returned within seconds. Decide the phase from the authoritative row:
			// reveal_confirmed → done; reveal_broadcasted → awaiting-confirmation (keep polling).
			const refreshed = await getProposalByActionId({ baseUrl, actionId })
			let nextPhase: BroadcastPhase = 'awaiting-confirmation'
			if (refreshed.ok) {
				applyProposal(refreshed.data)
				nextPhase = phaseForBroadcastStatus(refreshed.data.broadcastStatus, refreshed.data.status) ?? nextPhase
			} else {
				setResult(res.data)
				nextPhase = phaseForBroadcastStatus(res.data.broadcastStatus, res.data.proposalStatus) ?? nextPhase
			}
			setPhase(nextPhase)
			if (nextPhase === 'awaiting-confirmation') startConfirmationPoll()
		} finally {
			broadcastStarted.current = false
			inFlightActionIds.delete(actionId)
		}
	}

	return {
		phase,
		bundle,
		result,
		proposal,
		error,
		canResubmit: error?.recovery === 'resubmit-reveal',
		prepare,
		broadcast,
	}
}
