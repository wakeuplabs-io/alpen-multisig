import type { DecodedAction } from '@/api/signing'

/**
 * The authority whose signer set authorizes a decoded action. This is distinct from the target
 * only for a Security Council rotation: the action modifies the council but the Strata
 * Administrator signs it. The backend remains authoritative and independently enforces the
 * upstream mapping; this helper prevents presenting a misleading manual signing prompt.
 */
export function decodedActionAuthorizingAuthority(action: DecodedAction): string | null {
	switch (action.kind) {
		case 'multisig_update':
			return action.role === 'security_council' ? 'strata_admin' : action.role
		case 'vk_update':
			return action.authority
		// Upstream's `authorized_role()` for tx type 14 is `StrataAdministrator`: the council can
		// sweep to the safe harbour but must not also pick where the funds land.
		case 'safe_harbour_address_update':
			return 'strata_admin'
		case 'defcon_1':
		case 'defcon_3':
			return 'security_council'
		case 'cancel':
		case 'unknown':
			return null
	}
}
