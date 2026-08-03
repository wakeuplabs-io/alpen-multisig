export type { CreateProposalFormValues } from './create-proposal.schema'

export type ActionType = 'vk_update' | 'signer_update' | 'operator_set_update' | 'sequencer_key_update'

export type MultisigConfigSnapshot = {
	signers: string[]
	threshold: number
}

/**
 * What a validated draft resolves to before signing. The sighash alone is not enough for the
 * review step: no device ever renders it, so `seqNo` and `actionHex` travel with it to let the
 * UI resolve the values the device actually shows (canonical message and its SHA-256).
 */
export type ProposalPreview = {
	seqNo: number
	actionHex: string
	sighashHex: string
}
