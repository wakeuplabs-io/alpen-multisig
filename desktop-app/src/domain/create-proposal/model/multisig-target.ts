import type { MultisigTargetAuthority } from '@/api/action-builder'
import type { ActionType } from './create-proposal.types'

/**
 * The authority a create-proposal draft targets — Constraint 2: decided by the **action**, never
 * the session. Every consumer that needs to know what the form is validating against, building
 * for, or previewing derives it from this one function.
 *
 * Read-side twin: `@/lib/multisig-update-target.ts` (`multisigUpdateTargetAuthority`), which reads
 * a *decoded* action instead of a form draft. It lives next to `ActionType` rather than beside its
 * twin because it needs the domain's `ActionType`, and `lib → domain` would invert the layering.
 * Unifying the two later is a deliberate move, not a discovery — each file names the other.
 */
export function multisigTargetAuthority(
	actionType: ActionType,
	sessionAuthority: MultisigTargetAuthority,
): MultisigTargetAuthority {
	return actionType === 'council_signer_update' ? 'security_council' : sessionAuthority
}
