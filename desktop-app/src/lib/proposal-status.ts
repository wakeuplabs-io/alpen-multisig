import type { ProposalStatus } from '@/api/proposals'

/** `awaiting_enactment` is a UI-only refinement of `approved`, not a backend status. */
export type DisplayStatus = ProposalStatus | 'awaiting_enactment'

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
	pending: { bg: '#fffbeb', text: '#111827', border: '#f0cfa0', dot: '#111827', label: 'Pending' },
	approved: { bg: '#eff6ff', text: '#2563eb', border: '#bfdbfe', dot: '#2563eb', label: 'Approved' },
	awaiting_enactment: {
		bg: '#f0fdf9',
		text: '#0f766e',
		border: '#99f6e4',
		dot: '#0f9d7a',
		label: 'Awaiting enactment',
	},
	enacted: { bg: '#ecfdf5', text: '#059669', border: '#a7f3d0', dot: '#059669', label: 'Enacted' },
	canceled: { bg: '#fef2f2', text: '#dc2626', border: '#fecaca', dot: '#dc2626', label: 'Canceled' },
	expired: { bg: '#f9fafb', text: '#6b7280', border: '#e5e7eb', dot: '#6b7280', label: 'Expired' },
}
