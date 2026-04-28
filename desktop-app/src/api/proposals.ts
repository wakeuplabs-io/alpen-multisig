import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'

export type Proposal = {
	actionId: string
	seqNo: number
	authority: string
	status: string
	actionHex: string
	signatures: Array<{
		signerPubkey: string
		signatureHex: string
	}>
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
	status?: string
}

export function createProposal(input: CreateProposalInput): Promise<ApiResult<Proposal>> {
	return tauriCall<Proposal>('proposals_create', { input })
}

export function listProposals(input: ListProposalsInput): Promise<ApiResult<Proposal[]>> {
	return tauriCall<Proposal[]>('proposals_list', { input })
}
