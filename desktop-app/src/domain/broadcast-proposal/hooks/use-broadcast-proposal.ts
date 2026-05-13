import { useState } from 'react'
import { broadcastProposal, prepareBroadcast } from '@/api/proposals'
import type { BroadcastInput, BroadcastResult, PrepareBroadcastResult } from '@/api/proposals'

import type { BroadcastPhase } from '../model/broadcast-proposal'

type UseBroadcastProposalReturn = {
	phase: BroadcastPhase
	bundle: PrepareBroadcastResult | null
	result: BroadcastResult | null
	error: string | null
	prepare: () => Promise<void>
	broadcast: () => Promise<void>
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
		setResult(res.data)
		setPhase('done')
	}

	return { phase, bundle, result, error, prepare, broadcast }
}
