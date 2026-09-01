import type { Proposal } from '@/api/proposals'

/**
 * The Defcon 1 proposals that executed without changing anything.
 *
 * Activating the safe harbour is idempotent upstream: `activate_safe_harbour()` is a bare
 * `set_activated(true)` with no guard, on a flag that is never reset. So a Defcon 1 that enacts
 * after another one already has is accepted, runs, consumes a council sequence number and its
 * fees, and leaves the bridge exactly as it found it. `Enacted` is true of it — the ASM applied
 * the action — and this is the half that badge does not say.
 *
 * The earliest enacted Defcon 1 by sequence number is the one that activated the harbour; every
 * enacted one after it changed nothing. Only proposals this app knows about are considered, so an
 * activation performed outside it leaves no trace here and the answer errs towards saying nothing.
 *
 * V2 breaks that premise and has to revisit this. Defcon 3 activates the same flag, on a timelock,
 * so once it exists the earliest enacted Defcon 1 is no longer necessarily what turned the harbour
 * on — a Defcon 3 that matured first would have, and this would then call the wrong proposal the
 * activation and stay silent about a genuinely redundant one. The fix is the activation height
 * rather than the sequence number, and it is not worth writing before there is a second action
 * that can set the flag. See docs/specs/security-council-defcon.md.
 */
export function redundantDefcon1ActionIds(proposals: readonly Proposal[]): ReadonlySet<string> {
	const enacted = proposals
		.filter((proposal) => proposal.actionType === 'defcon_1' && proposal.status === 'enacted')
		.sort((a, b) => a.seqNo - b.seqNo)
	return new Set(enacted.slice(1).map((proposal) => proposal.actionId))
}
