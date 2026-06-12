export type { CreateProposalFormValues } from './create-proposal.schema'

export type ActionType = 'vk_update' | 'signer_update' | 'operator_set_update' | 'sequencer_key_update'

export type MultisigConfigSnapshot = {
	signers: string[]
	threshold: number
}
