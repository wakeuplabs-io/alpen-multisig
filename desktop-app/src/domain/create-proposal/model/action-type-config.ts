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
	defcon1: {
		actionType: 'defcon1',
		title: 'Defcon 1',
		description: 'Immediate bridge safe harbour activation. Irreversible.',
	},
	defcon3: {
		actionType: 'defcon3',
		title: 'Defcon 3',
		description: 'Timelocked bridge safe harbour activation.',
	},
}

/** Action types available to each authority, in display order. The first entry is the default selection. */
const ACTION_TYPES_BY_AUTHORITY: Record<string, ActionType[]> = {
	strata_admin: ['signer_update', 'vk_update', 'operator_set_update'],
	sequencer_manager: ['signer_update', 'sequencer_key_update'],
	alpen_admin: ['signer_update', 'vk_update'],
	security_council: ['defcon1', 'defcon3'],
}

export function getActionTypeOptions(authority: string): ActionTypeOption[] {
	const actionTypes = ACTION_TYPES_BY_AUTHORITY[authority] ?? ACTION_TYPES_BY_AUTHORITY.strata_admin
	return actionTypes.map((actionType) => ACTION_TYPE_OPTIONS[actionType])
}

export function getDefaultActionType(authority: string): ActionType {
	return getActionTypeOptions(authority)[0].actionType
}
