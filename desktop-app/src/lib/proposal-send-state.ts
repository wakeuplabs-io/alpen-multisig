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
	/** The chain moved past this proposal's sequence number. Nothing to press, ever again. */
	| { kind: 'superseded'; label: string; detail: string }

type SendStateInput = {
	status: ProposalStatus
	broadcastStatus: BroadcastStatus
	requiredSignatures: number
	signatures: ReadonlyArray<unknown>
}

/**
 * Definitions of every broadcast stage, in transition order. This table is what
 * the user-facing lifecycle doc describes, so keep the two in step.
 *
 * Each line says what the app did and what it has seen — never where a transaction is now. The
 * status here is the last one that was persisted; nothing re-reads the mempool once the send
 * screen is closed, so "it is in the mempool" was an assertion no code had checked, and it read
 * identically whether the transaction was propagating normally or had been dropped hours ago.
 */
const STAGE: Record<Exclude<BroadcastStatus, 'idle'>, { label: string; detail: string }> = {
	commit_broadcasted: {
		label: 'Commit sent',
		detail: 'The commit transaction was broadcast. The app has not seen it confirm.',
	},
	commit_confirmed: {
		label: 'Commit confirmed',
		detail: 'The commit transaction is mined. The reveal transaction goes out next.',
	},
	reveal_broadcasted: {
		label: 'Reveal sent',
		detail: 'The reveal transaction was broadcast. The app has not seen it confirm.',
	},
	reveal_confirmed: {
		label: 'Reveal confirmed — awaiting ASM enactment',
		detail:
			'Both transactions are on chain. Nothing left to send: the ASM applies the change if it accepts the action.',
	},
	failed: {
		label: 'Send failed',
		detail: 'The bundle was not broadcast. You can send it again.',
	},
}

/**
 * Said wherever the broadcast stage would be said, because it replaces it: this is the one
 * terminal state a signer is likely to have been waiting on when it arrives.
 *
 * Two ways to get here, and they are not the same thing to the person reading. A bundle whose
 * reveal was mined reached the chain and lost the race — it cost the commit and reveal fees, and
 * it is the case where the attribution rests on a sequence number rather than on a receipt, since
 * the ASM discards a refused action silently. A bundle that never confirmed never got that far.
 */
const SUPERSEDED_AFTER_CONFIRMATION = {
	label: 'Superseded',
	detail:
		'This transaction was mined, but another action had already used its sequence number, so the ASM did not apply it. The signatures are bound to that number, so it cannot be sent again — a replacement has to be created and signed. The commit and reveal fees were spent.',
}

const SUPERSEDED_BEFORE_CONFIRMATION = {
	label: 'Superseded',
	detail:
		'Another action used this sequence number before this proposal reached a block. The signatures are bound to that number, so it can no longer be sent — a replacement has to be created and signed.',
}

export function proposalSendState(proposal: SendStateInput): ProposalSendState {
	const isTerminal =
		proposal.status === 'enacted' ||
		proposal.status === 'canceled' ||
		proposal.status === 'expired' ||
		proposal.status === 'superseded'
	const hasQuorum =
		!isTerminal && (proposal.status === 'approved' || proposal.signatures.length >= proposal.requiredSignatures)

	// Only an approved proposal has a bundle to broadcast. Quorum alone is not
	// enough: the backend approves the proposal before the bundle exists.
	if (proposal.status === 'superseded') {
		const stage =
			proposal.broadcastStatus === 'reveal_confirmed' ? SUPERSEDED_AFTER_CONFIRMATION : SUPERSEDED_BEFORE_CONFIRMATION
		return { kind: 'superseded', ...stage }
	}
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
