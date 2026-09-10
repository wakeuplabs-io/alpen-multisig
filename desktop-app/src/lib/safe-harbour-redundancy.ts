import type { ActionType, ProposalStatus } from '@/api/proposals'

/**
 * The minimal shape needed to decide which enactments changed nothing.
 *
 * A structural subset of `Proposal` rather than the transport DTO itself — callers pass the real
 * object and tests build lean fixtures, the same trade `derive-proposal-actions.ts` writes down.
 * Deliberately not `Pick<Proposal, …>`: that would keep the import of the transport type this is
 * written to avoid. `ActionType` and `ProposalStatus` still come from there because those unions
 * have no other home.
 *
 * "Candidate", not "input": rows that never enacted are passed in and filtered out here.
 */
export type HarbourActivationCandidate = {
	actionId: string
	actionType: ActionType
	status: ProposalStatus
	activationHeight: number | null
	seqNo: number
}

/** Both levers set the same flag; only the delay differs. */
const HARBOUR_ACTIVATING_ACTIONS: readonly ActionType[] = ['defcon_1', 'defcon_3']

/**
 * The proposals that executed without changing anything.
 *
 * Activating the safe harbour is idempotent upstream: `activate_safe_harbour()` is a bare
 * `set_activated(true)` with no guard, on a flag that is never reset. So an action that enacts
 * after the harbour is already up is accepted, runs, consumes a council sequence number and its
 * fees, and leaves the bridge exactly as it found it. `Enacted` is true of it — the ASM applied
 * the action — and this is the half that badge does not say.
 *
 * The activator is the enacted harbour-activating proposal with the **lowest activation height**,
 * over both Defcon types. Not the lowest sequence number: Defcon 3 sets the same flag on a
 * timelock, so a Defcon 3 accepted earlier can mature later than a Defcon 1 accepted after it. The
 * two heights are comparable because a Defcon 1's lock period is `0`, which makes its activation
 * height its own reveal block.
 *
 * A null activation height is a missing observation, not a low one, and it is permanent — the
 * backend computes the height once, non-fatally, when the reveal confirms, and never retries. Such
 * a row is neither the activator nor redundant.
 *
 * That exclusion costs a badge V1 used to show, and the trade is deliberate. If the proposal that
 * really activated the harbour is the one whose height failed to compute, it drops out of the
 * ranking, the next-lowest known height is named the activator, and a proposal that genuinely
 * burned a sequence number for nothing goes unbadged. What it buys is that the badge never says
 * "changed nothing" about the activation itself: a row with no height cannot be ranked against one
 * that has a real number, and guessing its position from the sequence number is the very premise
 * this module was rewritten to drop — sound for a Defcon 1, whose height is its reveal block, and
 * false for a Defcon 3, whose height is the reveal plus a delay nobody recorded. Ordering the two
 * classes correctly needs a partial order, not a sort. The real fix is upstream, where the height
 * should be retried; see the debt in docs/specs/security-council-defcon-3-implementation.md §6.
 *
 * Only proposals this app knows about are considered, so the answer keeps erring towards saying
 * nothing.
 *
 * Ties are broken by sequence number. That is not a second ordering key — it decides between rows
 * the height has already declared indistinguishable, which is unobservable on chain (nothing
 * records which of two actions applied in the same block set the flag) and exists so the answer is
 * a function of the proposals rather than of the order they arrived in.
 */
export function changedNothingActionIds(proposals: readonly HarbourActivationCandidate[]): ReadonlySet<string> {
	const activations = proposals
		.filter(
			(proposal): proposal is HarbourActivationCandidate & { activationHeight: number } =>
				HARBOUR_ACTIVATING_ACTIONS.includes(proposal.actionType) &&
				proposal.status === 'enacted' &&
				proposal.activationHeight !== null,
		)
		.sort((a, b) => a.activationHeight - b.activationHeight || a.seqNo - b.seqNo)
	return new Set(activations.slice(1).map((proposal) => proposal.actionId))
}

/**
 * Whether a superseded proposal is one the harbour swallowed rather than one that lost a race.
 *
 * The two are indistinguishable in the proposal's own record — both end with the sequence number
 * consumed and the action gone from the queue — so the answer needs the bridge's live state, which
 * is why it takes the flag rather than reading it. Live is sound: activation is a `set_activated(true)`
 * on a flag with no reset and no de-escalating action upstream, so a harbour that is up now was up
 * when the rotation was mined, or came up after it.
 *
 * That "or came up after it" is the residual ambiguity: a rotation genuinely superseded by a rival
 * action, with the harbour raised afterwards, reads as swallowed. The attribution is then wrong and
 * the advice still right — a replacement really would be discarded — which is the trade recorded in
 * docs/specs/security-council-safe-harbour-address-phase-3.md §4.1.
 *
 * The action type is load-bearing and not a formality: every other action is applied whatever the
 * harbour is doing, and a superseded Defcon really did lose its sequence number to something else.
 */
export function harbourFrozeDestination(
	proposal: { actionType: ActionType; status: ProposalStatus },
	harbourActivated: boolean,
): boolean {
	return harbourActivated && proposal.status === 'superseded' && proposal.actionType === 'safe_harbour_address_update'
}
