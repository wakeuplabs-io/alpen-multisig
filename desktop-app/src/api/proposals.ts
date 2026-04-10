import { tauriCall } from './tauri-bridge'
import type { ApiResult, Proposal, ProposalSignature, ProposalStatus } from '@/types'

// Proposal commands map 1:1 to Tauri commands in main.rs.
// No token parameters — the session token is injected by Rust automatically.

// ─── Example (implemented) ────────────────────────────────────────────────────

export async function listProposals(status?: ProposalStatus): Promise<ApiResult<{ proposals: Proposal[] }>> {
	return tauriCall('list_proposals', { status })
}

// ─── TODO: remaining commands follow the same pattern ─────────────────────────
// Each one calls tauriCall('<command_name>', { ...args }) and the corresponding
// Tauri command in main.rs reads the session token from AppState internally.

export async function getProposal(_actionId: string): Promise<ApiResult<Proposal>> {
	throw new Error('TODO: invoke get_proposal Tauri command')
}

export type CreateProposalPayload = {
	seqNo: number
	actionPayload: unknown
}

export async function createProposal(
	_payload: CreateProposalPayload,
): Promise<ApiResult<{ actionId: string; proposal: Proposal }>> {
	throw new Error('TODO: invoke create_proposal Tauri command')
}

export async function submitSignature(
	_actionId: string,
	_signerPubkey: string,
	_signature: string,
): Promise<ApiResult<{ signatureId: string; quorumReached: boolean }>> {
	throw new Error('TODO: invoke submit_signature Tauri command')
}

export async function listSignatures(_actionId: string): Promise<ApiResult<{ signatures: ProposalSignature[] }>> {
	throw new Error('TODO: invoke list_signatures Tauri command')
}
