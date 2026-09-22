import type { ActionValidator } from './types'
import type { CreateProposalFormValues } from '../create-proposal.schema'
import { validateSignerUpdate } from './signer-update'
import { validateOperatorSetUpdate } from './operator-set-update'
import { validateSequencerKeyUpdate } from './sequencer-key-update'
import { validateSafeHarborAddressUpdate } from './safe-harbor-address-update'
import { validateVkUpdate } from './vk-update'
import { validateDefcon } from './defcon'

const actionValidators: Record<CreateProposalFormValues['actionType'], ActionValidator> = {
	signer_update: validateSignerUpdate,
	// Reused, not duplicated: the rules are identical, and only the signer set they answer against
	// changes — that set is supplied via `currentMultisigSigners`, retargeted in `create-proposal-form.tsx`.
	council_signer_update: validateSignerUpdate,
	operator_set_update: validateOperatorSetUpdate,
	sequencer_key_update: validateSequencerKeyUpdate,
	safe_harbour_address_update: validateSafeHarborAddressUpdate,
	vk_update: validateVkUpdate,
	defcon_1: validateDefcon('defcon_1'),
	defcon_3: validateDefcon('defcon_3'),
}

export function getActionValidator(actionType: CreateProposalFormValues['actionType']): ActionValidator {
	return actionValidators[actionType]
}
