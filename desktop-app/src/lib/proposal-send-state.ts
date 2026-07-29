import type { BroadcastStatus, ProposalStatus } from '@/api/proposals'

/**
 * What the proposal screens should offer for sending the commit+reveal bundle,
 * and what they should say about where that bundle currently is (#432).
 *
 * The dashboard and the detail screen each used to answer this themselves, and
 * they drifted: the detail screen offered "Send" on quorum alone, so it kept
 * offering it after the bundle had already been broadcast. Both read from here
 * now.
 */
export type ProposalSendState =
	/** Sending is not on the table: no quorum yet, or the proposal is terminal. */
	| { kind: 'unavailable' }
	/** Quorum reached and nothing broadcast yet — the Send button shows. */
	| { kind: 'ready' }
	/** Broadcast is under way on Bitcoin. No button; the label says which leg. */
	| { kind: 'in-flight'; label: string; detail: string }
	/** Reveal confirmed on Bitcoin, waiting for the ASM to apply the change. */
	| { kind: 'confirmed'; label: string; detail: string }
	/** Broadcast failed. The button comes back as a retry — the backend allows it. */
	| { kind: 'failed'; label: string; detail: string }

type SendStateInput = {
	status: ProposalStatus
	broadcastStatus: BroadcastStatus
	requiredSignatures: number
	signatures: ReadonlyArray<unknown>
}

/**
 * Definitions of every broadcast stage, in transition order. This table is what
 * the user-facing lifecycle doc describes, so keep the two in step.
 */
const STAGE: Record<Exclude<BroadcastStatus, 'idle'>, { label: string; detail: string }> = {
	commit_broadcasted: {
		label: 'Commit sent',
		detail: 'The commit transaction is in the mempool, waiting to be mined.',
	},
	commit_confirmed: {
		label: 'Commit confirmed',
		detail: 'The commit transaction is mined. The reveal transaction goes out next.',
	},
	reveal_broadcasted: {
		label: 'Reveal sent',
		detail: 'The reveal transaction is in the mempool, waiting to be mined.',
	},
	reveal_confirmed: {
		label: 'Reveal confirmed — awaiting ASM enactment',
		detail: 'Both transactions are on chain. Nothing left to send; the ASM applies the change after the delay.',
	},
	failed: {
		label: 'Send failed',
		detail: 'The bundle was not broadcast. You can send it again.',
	},
}

export function proposalSendState(proposal: SendStateInput): ProposalSendState {
	const isTerminal = proposal.status === 'enacted' || proposal.status === 'canceled' || proposal.status === 'expired'
	const hasQuorum =
		!isTerminal && (proposal.status === 'approved' || proposal.signatures.length >= proposal.requiredSignatures)

	// Only an approved proposal has a bundle to broadcast. Quorum alone is not
	// enough: the backend approves the proposal before the bundle exists.
	if (isTerminal || !hasQuorum || proposal.status !== 'approved') return { kind: 'unavailable' }

	switch (proposal.broadcastStatus) {
		case 'idle':
			return { kind: 'ready' }
		case 'failed':
			return { kind: 'failed', ...STAGE.failed }
		case 'reveal_confirmed':
			return { kind: 'confirmed', ...STAGE.reveal_confirmed }
		default:
			return { kind: 'in-flight', ...STAGE[proposal.broadcastStatus] }
	}
}

/** Whether the Send button is rendered at all — `ready` sends, `failed` retries. */
export function showsSendButton(state: ProposalSendState): boolean {
	return state.kind === 'ready' || state.kind === 'failed'
}

/** Button caption, so both screens word the retry identically. */
export function sendButtonLabel(state: ProposalSendState): string {
	return state.kind === 'failed' ? 'Retry send' : 'Send'
}
