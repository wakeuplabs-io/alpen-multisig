import type { ApiResult } from '@/types'
import { AuthRole } from '@/types'
import { authChallengeSchema, authSessionResultSchema, authSessionSchema } from '@/api/ipc-schemas'
import { tauriCall } from '@/api/tauri-bridge'
import type { SignatureFormat } from '@/wallet/types'

export { AuthRole } from '@/types'

export type StartChallengeInput = {
	role: AuthRole
	rpcUrl?: string
}

export type AuthChallenge = {
	challengeId: string
	challengeHex: string
	challengeMessage: string
	nonceHex: string
	domain: string
	role: AuthRole
	issuedAtUnixMs: number
	expiresAtUnixMs: number
	sessionId: string
}

export type CompleteAuthInput = {
	challengeId: string
	signerPubkeyHex: string
	signatureHex: string
	signatureFormat: SignatureFormat
}

export type AuthSession = {
	role: AuthRole
	signerPubkeyHex: string
	authenticatedAtUnixMs: number
	expiresAtUnixMs: number
	membershipFetchedAtUnixMs: number
}

export type AuthSessionResult = {
	authenticated: boolean
	session: AuthSession | null
}

export function authStartChallenge(input: StartChallengeInput): Promise<ApiResult<AuthChallenge>> {
	return tauriCall<AuthChallenge>('auth_start_challenge', { input }, authChallengeSchema)
}

export function authComplete(input: CompleteAuthInput): Promise<ApiResult<AuthSession>> {
	return tauriCall<AuthSession>('auth_complete', { input }, authSessionSchema)
}

export function authGetSession(): Promise<ApiResult<AuthSessionResult>> {
	return tauriCall<AuthSessionResult>('auth_get_session', undefined, authSessionResultSchema)
}

export function authLogout(): Promise<ApiResult<null>> {
	return tauriCall<null>('auth_logout')
}
