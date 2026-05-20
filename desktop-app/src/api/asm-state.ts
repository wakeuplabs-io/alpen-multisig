import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'
import { authorityMembershipsSchema, multisigConfigSchema } from '@/api/ipc-schemas'

export type MultisigConfig = {
	signers: string[]
	threshold: number
}

export type AuthorityMemberships = Record<string, boolean>

export function getMultisigConfig(authority: string): Promise<ApiResult<MultisigConfig>> {
	return tauriCall('get_multisig_config', { authority }, multisigConfigSchema)
}

export function checkAuthorityMemberships(pubkeyHex: string): Promise<ApiResult<AuthorityMemberships>> {
	return tauriCall('check_authority_memberships', { pubkeyHex }, authorityMembershipsSchema)
}
