import type { ApiResult } from '@/types'
import { tauriCall } from './tauri-bridge'

export type SighashResult = {
	sighashHex: string
	seqno: number
}

export type VerifyResult = {
	valid: boolean
	signaturesVerified: number
	thresholdRequired: number
}

export function computeSighash(seqno: number, actionHex: string): Promise<ApiResult<SighashResult>> {
	return tauriCall<SighashResult>('compute_sighash', { seqno, action_hex: actionHex })
}

export function verifyThreshold(
	publicKeysHex: string[],
	threshold: number,
	signaturesHex: string[],
	sighashHex: string,
): Promise<ApiResult<VerifyResult>> {
	return tauriCall<VerifyResult>('verify_threshold', {
		public_keys_hex: publicKeysHex,
		threshold,
		signatures_hex: signaturesHex,
		sighash_hex: sighashHex,
	})
}
