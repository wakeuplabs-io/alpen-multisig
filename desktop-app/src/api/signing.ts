import { z } from 'zod'
import type { ApiResult } from '@/types'
import { tauriCall } from './tauri-bridge'
import { decodedActionSchema, sighashResultSchema, signatureResultSchema, verifyResultSchema } from './ipc-schemas'

export type SighashResult = {
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

export type DecodedAction =
	| { kind: 'multisig_update'; role: string; addKeys: string[]; removeKeys: string[]; newThreshold: number }
	| { kind: 'vk_update'; authority: string; typeId: number; conditionHex: string }
	| { kind: 'defcon_1' }
	| { kind: 'defcon_3' }
	| { kind: 'unknown'; rawHex: string }

export function decodeActionHex(actionHex: string): Promise<ApiResult<DecodedAction>> {
	return tauriCall<DecodedAction>('decode_action_hex', { actionHex }, decodedActionSchema)
}

export function computeSighash(seqno: number, actionHex: string): Promise<ApiResult<SighashResult>> {
	return tauriCall('compute_sighash', { seqno, actionHex }, sighashResultSchema)
}

/**
 * Canonical SPS-65 signing message (the exact text the device signs) for an action.
 * Used to show what the connected device displays: Trezor renders this text; Ledger
 * renders its SHA-256 ("Message hash"). The BIP-137 sighash never appears on-device.
 */
export function renderSigningMessage(seqno: number, actionHex: string): Promise<ApiResult<string>> {
	return tauriCall('render_signing_message', { seqno, actionHex }, z.string())
}

export function verifyThreshold(
	publicKeysHex: string[],
	threshold: number,
	signaturesHex: string[],
	sighashHex: string,
): Promise<ApiResult<VerifyResult>> {
	return tauriCall('verify_threshold', { publicKeysHex, threshold, signaturesHex, sighashHex }, verifyResultSchema)
}

export function signSighashMock(secretKeyHex: string, sighashHex: string): Promise<ApiResult<SignatureResult>> {
	return tauriCall('sign_sighash_mock', { secretKeyHex, sighashHex }, signatureResultSchema)
}
