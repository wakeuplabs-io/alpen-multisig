import { api } from './client'
import type { ApiResult, AuthChallenge, Session } from '@/types'

export async function getChallenge(
  signerPubkey: string,
  authority: string
): Promise<ApiResult<AuthChallenge>> {
  return api.get(`/auth/challenge?signer_pubkey=${signerPubkey}&authority=${authority}`)
}

export type CreateSessionPayload = {
  ephemeralPubkey: string
  nonce: string
  attestationSignature: string
  signerPubkey: string
  authority: string
}

export async function createSession(
  payload: CreateSessionPayload
): Promise<ApiResult<Session>> {
  return api.post('/auth/session', {
    ephemeral_pubkey: payload.ephemeralPubkey,
    nonce: payload.nonce,
    attestation_signature: payload.attestationSignature,
    signer_pubkey: payload.signerPubkey,
    authority: payload.authority,
  })
}

export async function deleteSession(token: string): Promise<ApiResult<void>> {
  return api.delete('/auth/session', token)
}
