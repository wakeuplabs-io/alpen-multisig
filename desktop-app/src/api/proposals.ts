import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'

export type ProposalStatus = 'pending' | 'approved' | 'enacted' | 'canceled' | 'expired'

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
