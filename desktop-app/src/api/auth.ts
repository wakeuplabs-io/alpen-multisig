import { tauriCall } from './tauri-bridge'
import type { ApiResult, AuthChallenge, SessionInfo } from '@/types'

// Auth commands map 1:1 to Tauri commands in main.rs.
// No token parameters — the session token is managed by Rust and never touches JS.

export async function getChallenge(
	signerPubkey: string,
	authority: string
): Promise<ApiResult<AuthChallenge>> {
	return tauriCall<AuthChallenge>('get_challenge', { pubkey: signerPubkey, authority })
}

export type CreateSessionPayload = {
	ephemeralPubkey: string
	nonce: string
	attestationSignature: string
	signerPubkey: string
	authority: string
}

// Returns SessionInfo (no token) — the bearer token stays in Rust's AppState.
export async function createSession(
	payload: CreateSessionPayload
): Promise<ApiResult<SessionInfo>> {
	return tauriCall<SessionInfo>('create_session', {
		payload: {
			ephemeral_pubkey: payload.ephemeralPubkey,
			nonce: payload.nonce,
			attestation_signature: payload.attestationSignature,
			signer_pubkey: payload.signerPubkey,
			authority: payload.authority,
		},
	})
}

export async function deleteSession(): Promise<ApiResult<void>> {
	return tauriCall<void>('delete_session')
}
