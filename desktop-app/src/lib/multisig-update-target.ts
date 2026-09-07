import type { DecodedAction } from '@/api/signing'

/**
 * The authority a decoded action modifies — the target, never the session (Constraint 2).
 *
 * `null` for every action that has no target authority. For the three self-rotating updates this
 * equals the proposal's own authority; for tx type 15 it does not, which is the whole of slice V3.
 *
 * Returns the authority rather than a boolean deliberately: it is the same value Phase 3's
 * read-side retarget will hand to `getMultisigConfig`.
 */
export function multisigUpdateTargetAuthority(action: DecodedAction | null): string | null {
	return action !== null && action.kind === 'multisig_update' ? action.role : null
}
