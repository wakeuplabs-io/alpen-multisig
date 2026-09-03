import { DEFCON_COPY } from '@/lib/defcon-copy'
import type { ActionType } from './create-proposal.types'

export type ActionTypeOption = {
	actionType: ActionType
	title: string
	description: string
}

const ACTION_TYPE_OPTIONS: Record<ActionType, ActionTypeOption> = {
	vk_update: {
		actionType: 'vk_update',
		title: 'Verification key update',
		description: 'Rotate the Alpen VK.',
	},
	signer_update: {
		actionType: 'signer_update',
		title: 'Signer update',
		description: 'Add / remove signers or change threshold.',
	},
	operator_set_update: {
		actionType: 'operator_set_update',
		title: 'Bridge Operator update',
		description: 'Add operators by key or remove by index.',
	},
	sequencer_key_update: {
		actionType: 'sequencer_key_update',
		title: 'Sequencer key update',
		description: 'Rotate the sequencer public key.',
	},
	// Both Defcon cards read their words from the shared copy table, so the menu cannot describe
	// one lever with the other's wording — the mistake Constraint 5 exists to prevent.
	defcon_1: {
		actionType: 'defcon_1',
		title: DEFCON_COPY.defcon_1.menuTitle,
		description: DEFCON_COPY.defcon_1.menuDescription,
	},
	defcon_3: {
		actionType: 'defcon_3',
		title: DEFCON_COPY.defcon_3.menuTitle,
		description: DEFCON_COPY.defcon_3.menuDescription,
	},
}

/** Action types available to each authority, in display order. The first entry is the default selection. */
const ACTION_TYPES_BY_AUTHORITY: Record<string, ActionType[]> = {
	strata_admin: ['signer_update', 'vk_update', 'operator_set_update'],
	sequencer_manager: ['signer_update', 'sequencer_key_update'],
	alpen_admin: ['signer_update', 'vk_update'],
	// Defcon 1 first, and therefore the council's default selection: the immediate lever is the one
	// an emergency reaches for, and a default is a decision, not an accident.
	security_council: ['defcon_1', 'defcon_3'],
}

/** The action type's display title — the same string the selection card carries. */
export function actionTypeTitle(actionType: ActionType): string {
	return ACTION_TYPE_OPTIONS[actionType].title
}

export function getActionTypeOptions(authority: string): ActionTypeOption[] {
	const actionTypes = ACTION_TYPES_BY_AUTHORITY[authority] ?? ACTION_TYPES_BY_AUTHORITY.strata_admin
	return actionTypes.map((actionType) => ACTION_TYPE_OPTIONS[actionType])
}

export function getDefaultActionType(authority: string): ActionType {
	return getActionTypeOptions(authority)[0].actionType
}
