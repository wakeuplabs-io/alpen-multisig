export type { CreateProposalFormValues } from './create-proposal.schema'

export type ActionType = 'vk_update' | 'signer_update' | 'operator_set_update' | 'sequencer_key_update'

export type MultisigConfigSnapshot = {
	signers: string[]
	threshold: number
}

/**
 * What a validated draft resolves to before signing: exactly what the review step needs to
 * resolve the values the device actually shows (canonical message and its SHA-256). The sighash
 * is deliberately absent — no device renders it, and signing recomputes it (#402).
 */
export type ProposalPreview = {
	seqNo: number
	actionHex: string
}
