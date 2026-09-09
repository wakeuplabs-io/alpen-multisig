import type { Proposal } from '@/api/proposals'
import type { DecodedProposalData } from '@/domain/proposal-detail/hooks/use-decoded-proposal'
import { SignerSetChangeTable } from '@/domain/signer-set-change/components/signer-set-change-table'
import { SafeHarbourChangeTable } from '@/domain/safe-harbour-change/components/safe-harbour-change-table'
import { buildProposalTitle } from '@/lib/proposal-title'

type Props = {
	proposal: Proposal
	decodedData: DecodedProposalData
}

export function CancelTargetSummary({ proposal, decodedData }: Props) {
	const change = decodedData.signerSetChange

	const changeLabel = (() => {
		// The screen where an administrator stands a rotation down has to name what it installs.
		if (decodedData.safeHarbourChange !== null) return 'Change sweep destination'
		if (change === null) return null
		const added = change.rows.filter((r) => r.isAdded).length
		const removed = change.rows.filter((r) => r.isRemoved).length
		if (added > 0 && removed === 0) return `Add ${added} signer${added > 1 ? 's' : ''}`
		if (removed > 0 && added === 0) return `Remove ${removed} signer${removed > 1 ? 's' : ''}`
		if (added > 0 && removed > 0) return 'Rotate signers'
		return null
	})()

	return (
		<div className="overflow-hidden rounded-xl border border-[#e5e7eb] bg-white shadow-sm">
			<div className="border-b border-[#f3f4f6] px-6 py-4">
				<p className="m-0 text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]">
					Proposal being cancelled
				</p>
			</div>

			<div className="px-6 py-5 space-y-1">
				{/* `changeLabel` only ever resolves for a multisig update, so without the fallback the card
				    headed "Proposal being cancelled" identified a Defcon 3 as a bare `Proposal #N` — on the
				    one screen where a council signer decides whether to cancel the sweep of the bridge. */}
				<p className="m-0 text-body-lg font-medium text-[#0a0a0a]">{changeLabel ?? buildProposalTitle(proposal)}</p>
				<p className="m-0 text-label text-[#6b7280]">
					#{proposal.seqNo} · {proposal.authority}
				</p>
			</div>

			{decodedData.isLoading && (
				<div className="border-t border-[#f3f4f6] px-6 py-4">
					<div className="h-3 w-40 animate-pulse rounded bg-[#f3f4f6]" />
				</div>
			)}

			{!decodedData.isLoading && change !== null && (
				<div className="overflow-hidden border-t border-[#f3f4f6]">
					<SignerSetChangeTable change={change} />
				</div>
			)}

			{!decodedData.isLoading && decodedData.safeHarbourChange !== null && (
				<div className="overflow-hidden border-t border-[#f3f4f6]">
					<SafeHarbourChangeTable change={decodedData.safeHarbourChange} />
				</div>
			)}
		</div>
	)
}
