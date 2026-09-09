import type { ActionValidator } from './types'
import type { CreateProposalFormValues } from '../create-proposal.schema'
import { validateSignerUpdate } from './signer-update'
import { validateOperatorSetUpdate } from './operator-set-update'
import { validateSequencerKeyUpdate } from './sequencer-key-update'
import { validateSafeHarbourAddressUpdate } from './safe-harbour-address-update'
import { validateVkUpdate } from './vk-update'
import { validateDefcon1 } from './defcon-1'
import { validateDefcon3 } from './defcon-3'

const actionValidators: Record<CreateProposalFormValues['actionType'], ActionValidator> = {
	signer_update: validateSignerUpdate,
	// Reused, not duplicated: the rules are identical, and only the signer set they answer against
	// changes — that set is supplied via `currentMultisigSigners`, retargeted in `create-proposal-form.tsx`.
	council_signer_update: validateSignerUpdate,
	operator_set_update: validateOperatorSetUpdate,
	sequencer_key_update: validateSequencerKeyUpdate,
	safe_harbour_address_update: validateSafeHarbourAddressUpdate,
	vk_update: validateVkUpdate,
	defcon_1: validateDefcon1,
	defcon_3: validateDefcon3,
}

export function getActionValidator(actionType: CreateProposalFormValues['actionType']): ActionValidator {
	return actionValidators[actionType]
}
