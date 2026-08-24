import type { Proposal } from '@/api/proposals'
import { inferProposalTypeLabel } from '@/lib/proposal-type-label'

/**
 * The heading a proposal is shown under.
 *
 * When the author wrote a title, that is what people are looking for — it is the only part of a
 * proposal that says *why* the change is being made. Everything else on a card is machine-derived,
 * and with several proposals of the same type queued at once the derived label alone cannot tell
 * them apart.
 *
 * The title is unsigned coordination metadata, so it never stands on its own: callers keep the
 * `#seqNo · type` line beside it, and a signer always sees the real action type on screen.
 *
 * Falls back to the derived label when there is no title — proposals created before the field
 * existed, and cancel proposals, which are labelled from their target.
 */
export function buildProposalTitle(proposal: Proposal): string {
	const authored = proposal.title?.trim()
	if (authored) return authored
	return derivedProposalLabel(proposal)
}

/** The machine-derived label: what every screen showed before titles were persisted. */
export function derivedProposalLabel(proposal: Proposal): string {
	if (proposal.kind === 'cancel') return `Cancel #${proposal.seqNo}`
	return `Proposal #${proposal.seqNo} - ${inferProposalTypeLabel(proposal)}`
}
