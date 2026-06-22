import type { ActionValidator } from './types'
import type { CreateProposalFormValues } from '../create-proposal.schema'
import { validateSignerUpdate } from './signer-update'
import { validateOperatorSetUpdate } from './operator-set-update'
import { validateSequencerKeyUpdate } from './sequencer-key-update'
import { validateVkUpdate } from './vk-update'

const actionValidators: Record<CreateProposalFormValues['actionType'], ActionValidator> = {
	signer_update: validateSignerUpdate,
	operator_set_update: validateOperatorSetUpdate,
	sequencer_key_update: validateSequencerKeyUpdate,
	vk_update: validateVkUpdate,
}

export function getActionValidator(actionType: CreateProposalFormValues['actionType']): ActionValidator {
	return actionValidators[actionType]
}
