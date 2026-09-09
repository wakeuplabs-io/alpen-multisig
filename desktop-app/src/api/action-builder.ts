import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'
import { buildActionHexResponseSchema } from '@/api/ipc-schemas'

// The four authorities a multisig config update can target. `payout_admin` is deliberately not a
// member: it has no ASM role (orchestrator-be/src/infrastructure/asm_role_membership.rs:277-280)
// and dies in the codec (desktop-app/src-tauri/src/infrastructure/action_codec.rs:154-156).
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

/** Defcon 1 carries no payload, so its action hex is a constant the Rust side encodes. */
export type BuildSafeHarbourAddressUpdateHexInput = {
	/** A bech32m P2TR address on the active network. Rust converts it to the BOSD descriptor. */
	address: string
}

export function buildSafeHarbourAddressUpdateHex(
	input: BuildSafeHarbourAddressUpdateHexInput,
): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_safe_harbour_address_update_hex', { input }, buildActionHexResponseSchema)
}

export function buildDefcon1ActionHex(): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_defcon_1_action_hex', {}, buildActionHexResponseSchema)
}

/** Defcon 3 is the same payload-less shape; only the delay before it takes effect differs. */
export function buildDefcon3ActionHex(): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_defcon_3_action_hex', {}, buildActionHexResponseSchema)
}

export function buildCancelActionHex(targetActionHex: string): Promise<ApiResult<BuildActionHexResponse>> {
	return tauriCall('build_cancel_action_hex', { targetActionHex }, buildActionHexResponseSchema)
}
