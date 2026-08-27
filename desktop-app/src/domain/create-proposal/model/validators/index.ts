import type { ActionValidator } from './types'
import type { CreateProposalFormValues } from '../create-proposal.schema'
import { validateSignerUpdate } from './signer-update'
import { validateOperatorSetUpdate } from './operator-set-update'
import { validateSequencerKeyUpdate } from './sequencer-key-update'
import { validateVkUpdate } from './vk-update'
import { validateDefcon1 } from './defcon-1'

const actionValidators: Record<CreateProposalFormValues['actionType'], ActionValidator> = {
	signer_update: validateSignerUpdate,
	operator_set_update: validateOperatorSetUpdate,
	sequencer_key_update: validateSequencerKeyUpdate,
	vk_update: validateVkUpdate,
	defcon_1: validateDefcon1,
}

export function getActionValidator(actionType: CreateProposalFormValues['actionType']): ActionValidator {
	return actionValidators[actionType]
}
