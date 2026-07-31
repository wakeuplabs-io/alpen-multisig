import type { ApiResult } from '@/types'
import { broadcastResultSchema, proposalSchema } from '@/api/ipc-schemas'
import { tauriCall } from '@/api/tauri-bridge'
import { z } from 'zod'

export type ProposalStatus = 'pending' | 'approved' | 'enacted' | 'canceled' | 'expired'

export type BroadcastStatus =
	'idle' | 'commit_broadcasted' | 'commit_confirmed' | 'reveal_broadcasted' | 'reveal_confirmed' | 'failed'

export type ProposalKind = 'update' | 'cancel'

export type ActionType =
	'multisig_update' | 'vk_update' | 'operator_set_update' | 'sequencer_key_update' | 'cancel' | 'unknown'

export type CancelProposalSummary = {
	actionId: string
	status: ProposalStatus
	signatures: Array<{ signerPubkey: string; signatureHex: string }>
	requiredSignatures: number
}

export type Proposal = {
	actionId: string
	seqNo: number
	authority: string
	status: ProposalStatus
	requiredSignatures: number
	actionHex: string
	actionType: ActionType
	signatures: Array<{
		signerPubkey: string
		signatureHex: string
	}>
	broadcastStatus: BroadcastStatus
	commitTxid?: string
	revealTxid?: string
	broadcastError?: string
	kind: ProposalKind
	targetActionId: string | null
	activationHeight: number | null
	updateIdInQueue: number | null
	cancelProposal: CancelProposalSummary | null
	createdAtMs: number
	expiresAtMs: number
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
	commitTxid?: string
	revealTxid?: string
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
	return tauriCall('proposals_create', { input }, proposalSchema)
}

export function listProposals(input: ListProposalsInput): Promise<ApiResult<Proposal[]>> {
	return tauriCall('proposals_list', { input }, z.array(proposalSchema))
}

export function getProposalByActionId(input: GetProposalInput): Promise<ApiResult<Proposal>> {
	return tauriCall('proposals_get', { input }, proposalSchema)
}

// Pre-broadcast guard for a Cancel proposal: is its target action still queued on the ASM?
export function checkCancelTargetQueued(input: GetProposalInput): Promise<ApiResult<boolean>> {
	return tauriCall<boolean>('proposals_check_cancel_target_queued', { input })
}

export function approveProposal(input: ApproveProposalInput): Promise<ApiResult<Proposal>> {
	return tauriCall('proposals_approve', { input }, proposalSchema)
}

export type BroadcastInput = {
	baseUrl: string
	actionId: string
	feeRateSatPerKvb: number
}

export function prepareBroadcast(input: BroadcastInput): Promise<ApiResult<PrepareBroadcastResult>> {
	return tauriCall<PrepareBroadcastResult>('proposals_prepare_broadcast', { input })
}

export function broadcastProposal(input: BroadcastInput): Promise<ApiResult<BroadcastResult>> {
	return tauriCall('proposals_broadcast', { input }, broadcastResultSchema)
}

export type CreateCancelProposalInput = {
	baseUrl: string
	targetActionId: string
	seqNo: number
	actionHex: string
	signerPubkey: string
	signatureHex: string
}

export function createCancelProposal(input: CreateCancelProposalInput): Promise<ApiResult<Proposal>> {
	return tauriCall('proposals_create_cancel', { input }, proposalSchema)
}

export type BroadcastManualInput = {
	actionHex: string
	seqNo: number
	authority: string
	signatures: Array<{ signerPubkey: string; signatureHex: string }>
	feeRateSatPerKvb: number
}

export function prepareBroadcastManual(input: BroadcastManualInput): Promise<ApiResult<PrepareBroadcastResult>> {
	return tauriCall<PrepareBroadcastResult>('proposals_prepare_broadcast_manual', { input })
}

export function broadcastManualProposal(input: BroadcastManualInput): Promise<ApiResult<BroadcastResult>> {
	return tauriCall('proposals_broadcast_manual', { input }, broadcastResultSchema)
}

export type ReportBroadcastInput = {
	baseUrl: string
	actionId: string
	broadcastStatus: BroadcastStatus
	commitTxid?: string
	revealTxid?: string
	proposalStatus?: ProposalStatus
}

export function reportBroadcastProgress(input: ReportBroadcastInput): Promise<ApiResult<Proposal>> {
	return tauriCall('proposals_report_broadcast', { input }, proposalSchema)
}

export type ResolveBroadcastStatusInput = {
	commitTxid?: string
	revealTxid?: string
}

export type ResolveBroadcastStatusResult = {
	broadcastStatus: string
	commitConfirmations: number | null
	revealConfirmations: number | null
}

const resolveBroadcastStatusResultSchema = z.object({
	broadcastStatus: z.string(),
	commitConfirmations: z.number().nullable(),
	revealConfirmations: z.number().nullable(),
})

export function resolveBroadcastStatus(
	input: ResolveBroadcastStatusInput,
): Promise<ApiResult<ResolveBroadcastStatusResult>> {
	return tauriCall('proposals_resolve_broadcast_status', { input }, resolveBroadcastStatusResultSchema)
}
