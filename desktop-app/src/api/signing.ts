import type { ApiResult } from '@/types'
import { tauriCall } from './tauri-bridge'

export type SighashResult = {
	sighashHex: string
	seqno: number
}

type RawSighashResult = {
	sighashHex: string
	seqno: number
}

export type VerifyResult = {
	valid: boolean
	signaturesVerified: number
	thresholdRequired: number
}

export type SignatureResult = {
	publicKeyHex: string
	signatureHex: string
}

type RawSignatureResult = {
	publicKeyHex: string
	signatureHex: string
}

export type DecodedAction =
	| { kind: 'multisig_update'; role: string; addKeys: string[]; removeKeys: string[]; newThreshold: number }
	| { kind: 'unknown'; rawHex: string }

export function decodeActionHex(actionHex: string): Promise<ApiResult<DecodedAction>> {
	return tauriCall<DecodedAction>('decode_action_hex', { actionHex })
}

export function computeSighash(seqno: number, actionHex: string): Promise<ApiResult<SighashResult>> {
	return tauriCall<RawSighashResult>('compute_sighash', { seqno, actionHex }).then((result) => {
		if (!result.ok) {
			return result
		}
		return {
			ok: true,
			data: {
				sighashHex: result.data.sighashHex,
				seqno: result.data.seqno,
			},
		}
	})
}

export function verifyThreshold(
	publicKeysHex: string[],
	threshold: number,
	signaturesHex: string[],
	sighashHex: string,
): Promise<ApiResult<VerifyResult>> {
	return tauriCall<VerifyResult>('verify_threshold', {
		publicKeysHex,
		threshold,
		signaturesHex,
		sighashHex,
	})
}

export function signSighashMock(secretKeyHex: string, sighashHex: string): Promise<ApiResult<SignatureResult>> {
	return tauriCall<RawSignatureResult>('sign_sighash_mock', { secretKeyHex, sighashHex }).then((result) => {
		if (!result.ok) {
			return result
		}
		return {
			ok: true,
			data: {
				publicKeyHex: result.data.publicKeyHex,
				signatureHex: result.data.signatureHex,
			},
		}
	})
}
