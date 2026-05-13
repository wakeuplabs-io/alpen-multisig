import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'

export type ProposalStatus = 'pending' | 'approved' | 'enacted' | 'canceled' | 'expired'

export type BroadcastStatus =
	| 'idle'
	| 'commit_broadcasted'
	| 'commit_confirmed'
	| 'reveal_broadcasted'
	| 'reveal_confirmed'
	| 'failed'

export type Proposal = {
	actionId: string
	seqNo: number
	authority: string
	status: ProposalStatus
	requiredSignatures: number
	actionHex: string
	signatures: Array<{
		signerPubkey: string
		signatureHex: string
	}>
	broadcastStatus: BroadcastStatus
	commitTxid?: string
	revealTxid?: string
	broadcastError?: string
}

export type PrepareBroadcastResult = {
	actionId: string
	commitAddress: string
	commitAmountSats: number
	estimatedFeeSats: number
}

export type BroadcastResult = {
	actionId: string
	proposalStatus: ProposalStatus
	broadcastStatus: BroadcastStatus
	commitTxid: string
	revealTxid: string
}

export type GetNextSeqNoInput = {
	baseUrl: string
}

export type CreateProposalInput = {
	baseUrl: string
	seqNo: number
	actionHex: string
	signerPubkey: string
	signatureHex: string
}

export type ListProposalsInput = {
	baseUrl: string
	status?: ProposalStatus
}

export type GetProposalInput = {
	baseUrl: string
	actionId: string
}

export type ApproveProposalInput = {
	baseUrl: string
	actionId: string
	signerPubkey: string
	signatureHex: string
}

export function getNextSeqNo(input: GetNextSeqNoInput): Promise<ApiResult<number>> {
	return tauriCall<number>('proposals_get_next_seq_no', { input })
}

export function createProposal(input: CreateProposalInput): Promise<ApiResult<Proposal>> {
	return tauriCall<Proposal>('proposals_create', { input })
}

export function listProposals(input: ListProposalsInput): Promise<ApiResult<Proposal[]>> {
	return tauriCall<Proposal[]>('proposals_list', { input })
}

export function getProposalByActionId(input: GetProposalInput): Promise<ApiResult<Proposal>> {
	return tauriCall<Proposal>('proposals_get', { input })
}

export function approveProposal(input: ApproveProposalInput): Promise<ApiResult<Proposal>> {
	return tauriCall<Proposal>('proposals_approve', { input })
}

export type BroadcastInput = {
	baseUrl: string
	actionId: string
	btcRpcUrl: string
	btcRpcUser: string
	btcRpcPass: string
	btcWalletName?: string
	operatorSecretKeyHex: string
	magicBytesHex: string
	asmRpcUrl: string
	network?: string
	confirmPollIntervalMs?: number
	confirmTimeoutMs?: number
}

export function prepareBroadcast(input: BroadcastInput): Promise<ApiResult<PrepareBroadcastResult>> {
	return tauriCall<PrepareBroadcastResult>('proposals_prepare_broadcast', { input })
}

export function broadcastProposal(input: BroadcastInput): Promise<ApiResult<BroadcastResult>> {
	return tauriCall<BroadcastResult>('proposals_broadcast', { input })
}
