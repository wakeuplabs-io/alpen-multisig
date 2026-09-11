import type { ActionType } from '@/api/proposals'
import type { DecodedAction } from '@/api/signing'
import { multisigUpdateTargetAuthority } from '@/lib/multisig-update-target'

/**
 * The action type a decoded action belongs to.
 *
 * The offline path used to guess this from the first byte of the action hex —
 * `startsWith('01') ? 'vk_update' : 'multisig_update'` — so a Defcon 1 bundle was
 * labelled *Signer update* on the one screen a signer reaches when the
 * orchestrator cannot tell them what they are holding.
 *
 * Written as an exhaustive `Record` with no default arm on purpose: a mapping is
 * exactly where the next guess gets written, and this way a fifth decoded kind is
 * a compile error rather than a silent `multisig_update`. It still owns *kinds* —
 * it no longer owns *targets*: distinguishing a council rotation happens in the
 * function body below, since `kind` alone does not carry the role.
 */
const ACTION_TYPE_BY_KIND: Record<DecodedAction['kind'], ActionType> = {
	multisig_update: 'multisig_update',
	vk_update: 'vk_update',
	defcon_1: 'defcon_1',
	defcon_3: 'defcon_3',
	cancel: 'cancel',
	unknown: 'unknown',
}

export function actionTypeFromDecoded(action: DecodedAction): ActionType {
	// AC 5: the target lives in the action, and for tx type 15 it is not the authorizing
	// authority. Not expressible in ACTION_TYPE_BY_KIND — `kind` does not carry the role — so
	// this branch is guarded by a test rather than by the type system.
	if (multisigUpdateTargetAuthority(action) === 'security_council') return 'council_signer_update'
	return ACTION_TYPE_BY_KIND[action.kind]
}
