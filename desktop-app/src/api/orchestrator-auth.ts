import type { ApiResult } from '@/types'
import { AuthRole } from '@/types'
import type { SignatureFormat } from '@/wallet/types'
import { tauriCall } from '@/api/tauri-bridge'

export const ORCHESTRATOR_BASE_URL =
	import.meta.env.VITE_ORCHESTRATOR_BASE_URL ?? 'http://127.0.0.1:3000/api/v1'

export type OrchestratorAuthChallenge = {
	challengeId: string
	challengeHex: string
}

export type OrchestratorAuthSession = {
	token: string
	authority: string
	signerPubkey: string
	expiresAtUnixMs: number
}

type RawOrchestratorAuthChallenge = {
	challenge_id: string
	challenge_hex: string
}

type RawOrchestratorAuthSession = {
	token: string
	authority: string
	signer_pubkey: string
	expires_at_unix_ms: number
}

type StartOrchestratorAuthInput = {
	baseUrl: string
	authority: string
}

type CompleteOrchestratorAuthInput = {
	baseUrl: string
	challengeId: string
	signerPubkey: string
	signatureHex: string
	signatureFormat: SignatureFormat
}

export function authorityFromRole(role: AuthRole): string {
	switch (role) {
		case AuthRole.StrataAdministrator:
			return 'strata_admin'
		case AuthRole.StrataSequencerManager:
			return 'sequencer_manager'
		default:
			return 'strata_admin'
	}
}

export function orchestratorAuthStart(input: StartOrchestratorAuthInput): Promise<ApiResult<OrchestratorAuthChallenge>> {
	return tauriCall<RawOrchestratorAuthChallenge>('orchestrator_auth_start', { input }).then((result) => {
		if (!result.ok) {
			return result
		}
		return {
			ok: true,
			data: {
				challengeId: result.data.challenge_id,
				challengeHex: result.data.challenge_hex,
			},
		}
	})
}

export function orchestratorAuthComplete(input: CompleteOrchestratorAuthInput): Promise<ApiResult<OrchestratorAuthSession>> {
	return tauriCall<RawOrchestratorAuthSession>('orchestrator_auth_complete', { input }).then((result) => {
		if (!result.ok) {
			return result
		}
		return {
			ok: true,
			data: {
				token: result.data.token,
				authority: result.data.authority,
				signerPubkey: result.data.signer_pubkey,
				expiresAtUnixMs: result.data.expires_at_unix_ms,
			},
		}
	})
}

export function orchestratorAuthLogout(baseUrl: string): Promise<ApiResult<null>> {
	return tauriCall<null>('orchestrator_auth_logout', { baseUrl })
}

export function orchestratorAuthGetSession(): Promise<ApiResult<OrchestratorAuthSession | null>> {
	return tauriCall<RawOrchestratorAuthSession | null>('orchestrator_auth_get_session').then((result) => {
		if (!result.ok) {
			return result
		}
		if (result.data === null) {
			return {
				ok: true,
				data: null,
			}
		}
		return {
			ok: true,
			data: {
				token: result.data.token,
				authority: result.data.authority,
				signerPubkey: result.data.signer_pubkey,
				expiresAtUnixMs: result.data.expires_at_unix_ms,
			},
		}
	})
}
