import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'
import { buildActionHexResponseSchema } from '@/api/ipc-schemas'

export type BuildAdminMultisigUpdateHexInput = {
	role: 'strata_admin' | 'sequencer_manager' | 'alpen_admin'
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

export type BuildVkUpdateHexInput = {
	authority: string
	typeId: number
	conditionHex: string
}

export function buildVkUpdateHex(input: BuildVkUpdateHexInput): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_vk_update_hex', { input }, buildActionHexResponseSchema)
}

export type BuildOperatorSetUpdateHexInput = {
	addOperatorKeys: string[]
	removeOperatorIndices: number[]
}

export function buildOperatorSetUpdateHex(
	input: BuildOperatorSetUpdateHexInput,
): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_operator_set_update_hex', { input }, buildActionHexResponseSchema)
}

export type BuildSequencerKeyUpdateHexInput = {
	newPubKey: string
}

export function buildSequencerKeyUpdateHex(
	input: BuildSequencerKeyUpdateHexInput,
): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_sequencer_key_update_hex', { input }, buildActionHexResponseSchema)
}

/** Defcon 1 carries no payload, so its action hex is a constant the Rust side encodes. */
export function buildDefcon1ActionHex(): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_defcon_1_action_hex', {}, buildActionHexResponseSchema)
}

export function buildCancelActionHex(targetActionHex: string): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_cancel_action_hex', { targetActionHex }, buildActionHexResponseSchema)
}
