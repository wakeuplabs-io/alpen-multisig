import type { DecodedAction } from '@/api/signing'

/**
 * The authority a decoded action modifies — the target, never the session (Constraint 2).
 *
 * `null` for every action that has no target authority. For the three self-rotating updates this
 * equals the proposal's own authority; for tx type 15 (the council rotation) it does not.
 *
 * Returns the authority rather than a boolean deliberately: it is the value the read side hands
 * to `getMultisigConfig` to fetch the target's config.
 *
 * Write-side twin: `domain/create-proposal/model/multisig-target.ts` (`multisigTargetAuthority`),
 * which reads a form draft's `actionType` instead of a decoded action. Unifying the two later is a
 * deliberate move, not a discovery.
 */
export function multisigUpdateTargetAuthority(action: DecodedAction | null): string | null {
	return action !== null && action.kind === 'multisig_update' ? action.role : null
}
