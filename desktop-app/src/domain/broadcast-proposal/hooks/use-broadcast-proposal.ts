import { useState } from 'react'
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
export function mergeBroadcastWithProposal(
	broadcast: BroadcastResult,
	proposal: Proposal,
): BroadcastResult {
	return {
		actionId: proposal.actionId,
		proposalStatus: proposal.status,
		broadcastStatus: proposal.broadcastStatus,
		commitTxid: proposal.commitTxid ?? broadcast.commitTxid,
		revealTxid: proposal.revealTxid ?? broadcast.revealTxid,
	}
}

function buildBroadcastInput(baseUrl: string, actionId: string): BroadcastInput | string {
	const btcRpcUrl = import.meta.env.VITE_BTC_RPC_URL
	const btcRpcUser = import.meta.env.VITE_BTC_RPC_USER
	const btcRpcPass = import.meta.env.VITE_BTC_RPC_PASS
	const operatorSecretKeyHex = import.meta.env.VITE_OPERATOR_SECRET_KEY_HEX
	const magicBytesHex = import.meta.env.VITE_MAGIC_BYTES_HEX
	const asmRpcUrl = import.meta.env.VITE_ASM_RPC_URL

	if (!btcRpcUrl) return 'VITE_BTC_RPC_URL is not set'
	if (!btcRpcUser) return 'VITE_BTC_RPC_USER is not set'
	if (!btcRpcPass) return 'VITE_BTC_RPC_PASS is not set'
	if (!operatorSecretKeyHex) return 'VITE_OPERATOR_SECRET_KEY_HEX is not set'
	if (!magicBytesHex) return 'VITE_MAGIC_BYTES_HEX is not set'
	if (!asmRpcUrl) return 'VITE_ASM_RPC_URL is not set'

	return {
		baseUrl,
		actionId,
		btcRpcUrl,
		btcRpcUser,
		btcRpcPass,
		btcWalletName: import.meta.env.VITE_BTC_WALLET_NAME,
		operatorSecretKeyHex,
		magicBytesHex,
		asmRpcUrl,
		network: import.meta.env.VITE_BITCOIN_NETWORK,
	}
}

export function useBroadcastProposal(baseUrl: string, actionId: string): UseBroadcastProposalReturn {
	const [phase, setPhase] = useState<BroadcastPhase>('idle')
	const [bundle, setBundle] = useState<PrepareBroadcastResult | null>(null)
	const [result, setResult] = useState<BroadcastResult | null>(null)
	const [proposal, setProposal] = useState<Proposal | null>(null)
	const [error, setError] = useState<string | null>(null)

	async function prepare() {
		setPhase('preparing')
		setError(null)
		const input = buildBroadcastInput(baseUrl, actionId)
		if (typeof input === 'string') {
			setError(input)
			setPhase('error')
			return
		}
		const res = await prepareBroadcast(input)
		if (!res.ok) {
			setError(res.error)
			setPhase('error')
			return
		}
		setBundle(res.data)
		setPhase('confirming')
	}

	async function broadcast() {
		setPhase('broadcasting')
		setError(null)
		const input = buildBroadcastInput(baseUrl, actionId)
		if (typeof input === 'string') {
			setError(input)
			setPhase('error')
			return
		}
		const res = await broadcastProposal(input)
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
	}

	return { phase, bundle, result, proposal, error, prepare, broadcast }
}
