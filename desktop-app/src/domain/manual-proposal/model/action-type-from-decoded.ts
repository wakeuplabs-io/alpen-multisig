import type { ActionType } from '@/api/proposals'
import type { DecodedAction } from '@/api/signing'

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
 * a compile error rather than a silent `multisig_update`.
 */
const ACTION_TYPE_BY_KIND: Record<DecodedAction['kind'], ActionType> = {
	multisig_update: 'multisig_update',
	vk_update: 'vk_update',
	defcon_1: 'defcon_1',
	defcon_3: 'defcon_3',
	unknown: 'unknown',
}

export function actionTypeFromDecoded(action: DecodedAction): ActionType {
	return ACTION_TYPE_BY_KIND[action.kind]
}
