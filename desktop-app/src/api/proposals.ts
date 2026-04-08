import { api } from './client'
import type { ApiResult, Proposal, ProposalSignature, ProposalStatus } from '@/types'

export async function listProposals(
  token: string,
  status?: ProposalStatus
): Promise<ApiResult<{ proposals: Proposal[] }>> {
  const query = status ? `?status=${status}` : ''
  return api.get(`/proposals${query}`, token)
}

export async function getProposal(
  actionId: string,
  token: string
): Promise<ApiResult<Proposal>> {
  return api.get(`/proposals/${actionId}`, token)
}

export type CreateProposalPayload = {
  seqNo: number
  actionPayload: unknown
}

export async function createProposal(
  payload: CreateProposalPayload,
  token: string
): Promise<ApiResult<{ actionId: string; proposal: Proposal }>> {
  return api.post('/proposals', { seq_no: payload.seqNo, action_payload: payload.actionPayload }, token)
}

export async function submitSignature(
  actionId: string,
  signerPubkey: string,
  signature: string,
  token: string
): Promise<ApiResult<{ signatureId: string; quorumReached: boolean }>> {
  return api.post(`/proposals/${actionId}/signatures`, { signer_pubkey: signerPubkey, signature }, token)
}

export async function listSignatures(
  actionId: string,
  token: string
): Promise<ApiResult<{ signatures: ProposalSignature[] }>> {
  return api.get(`/proposals/${actionId}/signatures`, token)
}
