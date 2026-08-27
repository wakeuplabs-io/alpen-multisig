import { HexCopyRow } from './hex-copy-row'
import type { BroadcastError } from '../model/broadcast-proposal'

/**
 * The last resort: when no broadcaster could be reached, the signed commit and
 * reveal are handed over so they can go out through any Bitcoin RPC.
 *
 * Rendered on every send path, the offline route included — that route is the
 * one built for the case where the orchestrator is gone, and it was the one that
 * discarded the hex (AC 15b).
 *
 * Renders nothing unless the failure actually carries both transactions.
 */
export function SendManuallyPanel({ error }: { error: BroadcastError }) {
	if (error.recovery !== 'manual-broadcast' || error.commitTxHex == null || error.revealTxHex == null) return null

	return (
		<div className="space-y-3 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] p-4">
			<p className="m-0 text-body-sm font-medium text-[#111827]">Send manually</p>
			<p className="m-0 text-label text-[#6b7280]">
				Send the commit first, then the reveal, via any Bitcoin node (
				<code className="font-mono text-mono-sm">sendrawtransaction</code>).
			</p>
			<HexCopyRow label="Commit TX (send first)" value={error.commitTxHex} />
			<HexCopyRow label="Reveal TX (send second)" value={error.revealTxHex} />
		</div>
	)
}
