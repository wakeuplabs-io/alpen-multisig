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

import { getAdminWalletCanSign, walletSessionInit, walletSessionInitWatchOnly } from '@/api/admin-wallet'
import { adminWalletCapabilitySchema } from '@/api/ipc-schemas'
import { initAdminWalletForAdapter } from '@/contexts/session-provider-vendor-branch'
import type { WalletAdapter } from '@/wallet/types'

import type { BroadcastError, BroadcastPhase } from '../model/broadcast-proposal'
import { deriveBroadcastError } from '../model/broadcast-proposal'

export type SignerKind = 'hardware' | 'mnemonic'

const inFlightActionIds = new Set<string>()

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

			const refreshed = await getProposalByActionId({ baseUrl, actionId })
			if (refreshed.ok) {
				setProposal(refreshed.data)
				setResult(mergeBroadcastWithProposal(res.data, refreshed.data))
			} else {
				setResult(res.data)
			}
			setPhase('done')
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
