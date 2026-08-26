import type { Proposal } from '@/api/proposals'

export function inferProposalTypeLabel(proposal: Proposal): string {
	if (proposal.kind === 'cancel') return 'Cancel'
	if (proposal.actionType === 'vk_update') return 'Verification key update'
	if (proposal.actionType === 'operator_set_update') return 'Operator set update'
	if (proposal.actionType === 'sequencer_key_update') return 'Sequencer key update'
	if (proposal.actionType === 'defcon_1') return 'Defcon 1'
	if (proposal.actionType === 'multisig_update') {
		return proposal.authority.toLowerCase().includes('sequencer') ? 'Sequencer update' : 'Signer update'
	}
	return 'Unknown'
}
