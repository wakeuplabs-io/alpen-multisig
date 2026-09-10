import type { ActionType } from './create-proposal.types'

/**
 * Both signer-set actions — the administrator's own rotation and the council's — share one form
 * component (`SignerUpdateFormFields`), one schema branch, and one preview block, because the
 * payload shape is identical and only the target differs (Constraint 2,
 * `multisig-target.ts`). This predicate is the one place that pairs the two literals, so a third
 * signer-set action type — should one ever exist — is a single edit, not a grep across the form,
 * the preview and the config gate.
 */
export function isSignerUpdateActionType(actionType: ActionType): boolean {
	return actionType === 'signer_update' || actionType === 'council_signer_update'
}
