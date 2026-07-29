import type { BroadcastStatus, ProposalStatus } from '@/api/proposals'

// Minimal proposal shape needed to derive which signer actions are available.
// Kept as a structural subset of `Proposal` so callers pass the real domain
// object while tests can build lean fixtures.
export type ProposalActionInput = {
	status: ProposalStatus
	broadcastStatus: BroadcastStatus
	requiredSignatures: number
	signatures: ReadonlyArray<{ signerPubkey: string }>
}

export type ProposalActions = {
	isTerminal: boolean
	hasQuorum: boolean
	broadcastStarted: boolean
	alreadySigned: boolean
	canSign: boolean
	canBroadcast: boolean
}

// Single source of truth for signer-facing action availability on a proposal.
//
// Key rule: signing is gated on the broadcast NOT having started — never on
// quorum. A proposal that has reached quorum but is not yet broadcasted
// (`broadcastStatus === 'idle'`) must still allow an eligible, not-yet-signed
// signer to add their signature.
export function deriveProposalActions(proposal: ProposalActionInput, signerPubkey: string | null): ProposalActions {
	const collectedSignatures = proposal.signatures.length
	const isTerminal = proposal.status === 'enacted' || proposal.status === 'canceled' || proposal.status === 'expired'
	const hasQuorum =
		!isTerminal && (proposal.status === 'approved' || collectedSignatures >= proposal.requiredSignatures)
	const broadcastStarted = proposal.broadcastStatus !== 'idle'

	const alreadySigned =
		signerPubkey !== null &&
		proposal.signatures.some((s) => s.signerPubkey.toLowerCase() === signerPubkey.toLowerCase())

	const canSign = !isTerminal && !broadcastStarted && signerPubkey !== null && !alreadySigned

	// `failed` re-opens sending on purpose. The backend accepts a re-broadcast from
	// `Idle | Failed` and rejects every other state with a conflict, so hiding the
	// button after a failure would strand the user in a state the API can recover
	// from (#432).
	const canBroadcast =
		hasQuorum &&
		proposal.status === 'approved' &&
		(proposal.broadcastStatus === 'idle' || proposal.broadcastStatus === 'failed')

	return { isTerminal, hasQuorum, broadcastStarted, alreadySigned, canSign, canBroadcast }
}
