import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'

export type MultisigConfig = {
	signers: string[]
	threshold: number
}

export type AuthorityMemberships = Record<string, boolean>

export function getMultisigConfig(authority: string): Promise<ApiResult<MultisigConfig>> {
	return tauriCall<MultisigConfig>('get_multisig_config', { authority })
}

export function checkAuthorityMemberships(pubkeyHex: string): Promise<ApiResult<AuthorityMemberships>> {
	return tauriCall<AuthorityMemberships>('check_authority_memberships', { pubkeyHex })
}
