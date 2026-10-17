import type { BroadcastAttempt, BroadcastStatus } from '@/api/proposals'
import { normalizePubkey, truncatePubkey } from '@/lib/pubkey'

type BroadcastPairsInput = {
	broadcastStatus: BroadcastStatus
	revealTxid?: string
	broadcastAttempts?: ReadonlyArray<BroadcastAttempt> | null
}

export type BroadcastPairRow = {
	commitTxid: string
	revealTxid: string
	/** Who sent it: `You`, a short pubkey, or `Unknown` for pairs sent before the backend kept track. */
	sender: string
	/**
	 * `Mined` — the pair the ASM acted on. `Current` — the pair the proposal is tracking now.
	 * `Not used` — another pair was mined first. `Sent` — still out there, and may yet be mined.
	 */
	state: 'Mined' | 'Current' | 'Not used' | 'Sent'
}

/**
 * The commit + reveal pairs sent for a proposal, oldest first (#605).
 *
 * The orchestrator's row tracks one pair: the latest one sent, until a reveal is mined, and from
 * then on the mined one. The others stay listed because any of them may still be the first
 * reveal in a block — and once one is, the rest are ignored by the ASM on the sequence number.
 */
export function describeBroadcastPairs(proposal: BroadcastPairsInput, signerPubkey: string | null): BroadcastPairRow[] {
	const mined = proposal.broadcastStatus === 'reveal_confirmed'
	const own = signerPubkey === null ? null : normalizePubkey(signerPubkey)
	return (proposal.broadcastAttempts ?? []).map((attempt) => {
		const tracked = attempt.revealTxid.toLowerCase() === proposal.revealTxid?.toLowerCase()
		const sender =
			attempt.broadcasterPubkey === null
				? 'Unknown'
				: normalizePubkey(attempt.broadcasterPubkey) === own
					? 'You'
					: truncatePubkey(attempt.broadcasterPubkey)
		const state = tracked ? (mined ? 'Mined' : 'Current') : mined ? 'Not used' : 'Sent'
		return { commitTxid: attempt.commitTxid, revealTxid: attempt.revealTxid, sender, state }
	})
}
