import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'
import { buildActionHexResponseSchema } from '@/api/ipc-schemas'
import type { DefconLevel } from '@/lib/defcon-copy'

// The four authorities a multisig config update can target. `payout_admin` is deliberately not a
// member: it has no ASM role (`authority_to_role` in orchestrator-be's asm_role_membership.rs) and
// dies in the codec (desktop-app/src-tauri/src/infrastructure/action_codec.rs).
export type MultisigTargetAuthority = 'strata_admin' | 'sequencer_manager' | 'alpen_admin' | 'security_council'

export type BuildAdminMultisigUpdateHexInput = {
	role: MultisigTargetAuthority
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

export type BuildSafeHarborAddressUpdateHexInput = {
	/** A bech32m P2TR address on the active network. Rust converts it to the BOSD descriptor. */
	address: string
}

export function buildSafeHarborAddressUpdateHex(
	input: BuildSafeHarborAddressUpdateHexInput,
): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_safe_harbor_address_update_hex', { input }, buildActionHexResponseSchema)
}

const DEFCON_BUILD_COMMAND: Record<DefconLevel, string> = {
	defcon_1: 'build_defcon_1_action_hex',
	defcon_3: 'build_defcon_3_action_hex',
}

/**
 * Both Defcon levels carry no payload, so their action hex is a constant the Rust side encodes; only
 * the delay before it takes effect differs, and that is read live from the ASM, never encoded.
 */
export function buildDefconActionHex(level: DefconLevel): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall(DEFCON_BUILD_COMMAND[level], {}, buildActionHexResponseSchema)
}

export function buildCancelActionHex(targetActionHex: string): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_cancel_action_hex', { targetActionHex }, buildActionHexResponseSchema)
}
