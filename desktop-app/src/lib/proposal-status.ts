import type { ActionType, BroadcastStatus, ProposalStatus } from '@/api/proposals'

/**
 * `awaiting_enactment` and `quorum_reached` are UI-only refinements of `approved`, not backend
 * statuses. Resolve one with `proposalDisplayStatus` rather than reading `proposal.status`.
 */
export type DisplayStatus = ProposalStatus | 'awaiting_enactment' | 'quorum_reached'

export type StatusStyle = {
	bg: string
	text: string
	border: string
	dot: string
	label: string
}

/**
 * Badge palette for every proposal status, shared by the dashboard and the
 * detail screen — the two used to carry byte-identical copies of this table.
 *
 * `pending` is deliberately neutral (#416): waiting for signatures is a normal
 * step, and amber read as an alarm. `canceled` keeps its red, being the one
 * terminal negative outcome.
 */
export const PROPOSAL_STATUS_STYLE: Record<DisplayStatus, StatusStyle> = {
	pending: {
		bg: 'var(--color-highlight-surface)',
		text: 'var(--color-emphasis)',
		border: 'var(--color-accent-border)',
		dot: 'var(--color-emphasis)',
		label: 'Pending',
	},
	approved: { bg: '#eff6ff', text: '#2563eb', border: '#bfdbfe', dot: '#2563eb', label: 'Approved' },
	/**
	 * Defcon 1's carve-out from the Approved/Canceled lifecycle (PRD 06 §5.2.2): it reaches
	 * `approved` in the backend and must never render that word. Same palette as `approved` — it is
	 * the same lifecycle position, and only the word is carved out.
	 */
	quorum_reached: { bg: '#eff6ff', text: '#2563eb', border: '#bfdbfe', dot: '#2563eb', label: 'Quorum reached' },
	awaiting_enactment: {
		bg: '#f0fdf9',
		text: '#0f766e',
		border: '#99f6e4',
		dot: '#0f9d7a',
		label: 'Awaiting enactment',
	},
	enacted: { bg: '#ecfdf5', text: '#059669', border: '#a7f3d0', dot: '#059669', label: 'Enacted' },
	canceled: {
		bg: 'var(--color-danger-surface)',
		text: 'var(--color-danger)',
		border: 'var(--color-danger-border)',
		dot: 'var(--color-danger)',
		label: 'Canceled',
	},
	expired: { bg: '#f9fafb', text: '#6b7280', border: '#e5e7eb', dot: '#6b7280', label: 'Expired' },
	/**
	 * Same neutral palette as `expired`: both are proposals that ran out of a window rather than
	 * failing at anything. The difference is which window, and the card says so in words.
	 */
	superseded: { bg: '#f9fafb', text: '#6b7280', border: '#e5e7eb', dot: '#6b7280', label: 'Superseded' },
}

type DisplayStatusInput = {
	status: ProposalStatus
	broadcastStatus: BroadcastStatus
	actionType: ActionType
}

/**
 * The status a proposal is shown under, which is not always the status the backend stores.
 *
 * Both screens used to derive `awaiting_enactment` themselves, by different expressions that
 * happened to agree. They read this instead, so Defcon 1's carve-out lands in one place.
 */
export function proposalDisplayStatus(proposal: DisplayStatusInput): DisplayStatus {
	if (proposal.status !== 'approved') return proposal.status
	if (proposal.broadcastStatus === 'reveal_confirmed') return 'awaiting_enactment'
	return proposal.actionType === 'defcon_1' ? 'quorum_reached' : 'approved'
}

type ActivationCountdownInput = {
	status: ProposalStatus
	activationHeight: number | null
	actionType: ActionType
}

/**
 * Whether to show how many blocks are left before the ASM applies the update.
 *
 * The backend stores an activation height for every proposal — `reveal_block + lock_period` — and
 * Defcon 1's lock period is 0, so its height equals the block it already landed in. Counting down
 * to it describes a delay the emergency lever does not have. Keyed on the action and not on the
 * authority: Defcon 3 shares the authority and carries a real configurable depth.
 */
export function showsActivationCountdown(proposal: ActivationCountdownInput): boolean {
	return proposal.status === 'approved' && proposal.activationHeight !== null && proposal.actionType !== 'defcon_1'
}
