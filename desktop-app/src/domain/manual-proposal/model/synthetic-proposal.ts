import type { Proposal } from '@/api/proposals'
import type { ManualImportData, ManualSignature } from '@/domain/manual-proposal/model/manual-proposal.types'

type SyntheticProposalInput = {
	importData: ManualImportData
	localSignatures: ReadonlyArray<ManualSignature>
	requiredSignatures: number | null
	hasQuorum: boolean
}

// The proposal the manual generator hands to `ProposalDetail`. There is no orchestrator row behind
// it, so its status stands in for the one the backend would set: `approved` once quorum is
// collected. The detail view offers Send only for an approved proposal, so a hard-coded `pending`
// hid Send behind a "ready to broadcast" banner (#605).
export function buildSyntheticProposal({
	importData,
	localSignatures,
	requiredSignatures,
	hasQuorum,
}: SyntheticProposalInput): Proposal {
	return {
		actionId: `manual-${importData.sighashHex.slice(0, 16)}`,
		seqNo: importData.seqNo,
		authority: importData.authority,
		status: hasQuorum ? 'approved' : 'pending',
		requiredSignatures: requiredSignatures ?? 0,
		actionHex: importData.actionHex,
		actionType: importData.actionType,
		// The manual bundle carries no title: it is rebuilt from the sighash payload alone.
		title: null,
		signatures: localSignatures.map(({ signerPubkey, signatureHex }) => ({ signerPubkey, signatureHex })),
		broadcastStatus: 'idle',
		kind: importData.actionType === 'cancel' ? 'cancel' : 'update',
		targetActionId: null,
		activationHeight: null,
		updateIdInQueue: null,
		cancelProposal: null,
		isCancelable: false,
		broadcastClaimStale: false,
		createdAtMs: 0,
		updatedAtMs: 0,
		expiresAtMs: 0,
	}
}
