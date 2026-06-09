import { z } from 'zod'
import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'
import { authorityMembershipsSchema, currentVkSchema, multisigConfigSchema } from '@/api/ipc-schemas'

export type MultisigConfig = {
	signers: string[]
	threshold: number
}

export type AuthorityMemberships = Record<string, boolean>

export type CurrentVk = {
	typeId: number
	typeName: string
	conditionHex: string
}

export function getMultisigConfig(authority: string): Promise<ApiResult<MultisigConfig>> {
	return tauriCall('get_multisig_config', { authority }, multisigConfigSchema)
}

export function checkAuthorityMemberships(pubkeyHex: string): Promise<ApiResult<AuthorityMemberships>> {
	return tauriCall('check_authority_memberships', { pubkeyHex }, authorityMembershipsSchema)
}

export function getCurrentVk(): Promise<ApiResult<CurrentVk>> {
	return tauriCall('get_current_vk', {}, currentVkSchema)
}

export function getBitcoinBlockHeight(): Promise<ApiResult<number>> {
	return tauriCall('get_bitcoin_block_height', {}, z.number())
}
