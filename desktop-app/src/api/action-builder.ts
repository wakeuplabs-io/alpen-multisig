import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'
import { buildActionHexResponseSchema } from '@/api/ipc-schemas'

export type BuildAdminMultisigUpdateHexInput = {
	role: 'strata_admin'
	addKeys: string[]
	removeKeys: string[]
	newThreshold: number
}

export type BuildActionHexResponse = {
	actionHex: string
}

export function buildAdminMultisigUpdateHex(
	input: BuildAdminMultisigUpdateHexInput,
): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_admin_multisig_update_hex', { input }, buildActionHexResponseSchema)
}

export function buildCancelActionHex(
	targetActionHex: string,
	targetSeqNo: number,
): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_cancel_action_hex', { targetActionHex, targetSeqNo }, buildActionHexResponseSchema)
}
