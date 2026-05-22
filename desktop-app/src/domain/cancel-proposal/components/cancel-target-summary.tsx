import type { Proposal } from '@/api/proposals'
import type { DecodedProposalData } from '@/domain/proposal-detail/hooks/use-decoded-proposal'

type Props = {
	proposal: Proposal
	decodedData: DecodedProposalData
}

function truncatePubkey(pubkey: string): string {
	return `${pubkey.slice(0, 12)}…${pubkey.slice(-8)}`
}

export function CancelTargetSummary({ proposal, decodedData }: Props) {
	const change = decodedData.signerSetChange

	const changeLabel = (() => {
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
				<p className="m-0 text-[11px] font-semibold uppercase tracking-wider text-[#9ca3af]">
					Proposal being cancelled
				</p>
			</div>

			<div className="px-6 py-5 space-y-1">
				<p className="m-0 text-[15px] font-medium text-[#0a0a0a]">{changeLabel ?? `Proposal #${proposal.seqNo}`}</p>
				<p className="m-0 text-[12px] text-[#6b7280]">
					#{proposal.seqNo} · {proposal.authority}
				</p>
			</div>

			{decodedData.isLoading && (
				<div className="border-t border-[#f3f4f6] px-6 py-4">
					<div className="h-3 w-40 animate-pulse rounded bg-[#f3f4f6]" />
				</div>
			)}

			{!decodedData.isLoading && change !== null && (
				<div className="border-t border-[#f3f4f6] overflow-x-auto">
					<table className="w-full border-collapse text-[12px]">
						<thead>
							<tr className="border-b border-[#f3f4f6] bg-[#f9fafb]">
								<th className="px-6 py-2.5 text-left text-[11px] font-semibold uppercase tracking-wider text-[#9ca3af]">
									Before
								</th>
								<th className="px-6 py-2.5 text-left text-[11px] font-semibold uppercase tracking-wider text-[#9ca3af]">
									After
								</th>
							</tr>
						</thead>
						<tbody>
							{change.rows.map((row, i) => (
								<tr key={i} className="border-b border-[#f3f4f6] last:border-0">
									<td className="px-6 py-3">
										{row.inBefore ? (
											<span
												className={`font-mono leading-relaxed ${row.isRemoved ? 'text-[#dc2626] line-through' : 'text-[#374151]'}`}
											>
												{row.isRemoved && <span className="mr-1 text-[#dc2626]">−</span>}
												{truncatePubkey(row.pubkey)}
											</span>
										) : (
											<span className="text-[#9ca3af]">—</span>
										)}
									</td>
									<td className="px-6 py-3">
										{row.inAfter ? (
											<span
												className={`font-mono leading-relaxed ${row.isAdded ? 'font-medium text-[#0f9d7a]' : 'text-[#374151]'}`}
											>
												{row.isAdded && <span className="mr-1 text-[#0f9d7a]">+</span>}
												{truncatePubkey(row.pubkey)}
											</span>
										) : (
											<span className="text-[#9ca3af]">—</span>
										)}
									</td>
								</tr>
							))}
							<tr className="border-t border-[#e5e7eb] bg-[#f9fafb]">
								<td className="px-6 py-3">
									<span className="text-[11px] font-medium uppercase tracking-wide text-[#9ca3af]">Threshold </span>
									{change.thresholdBefore !== null ? (
										<span className="font-medium text-[#374151]">
											{change.thresholdBefore} of {change.rows.filter((r) => r.inBefore).length}
										</span>
									) : (
										<span className="text-[#9ca3af]">—</span>
									)}
								</td>
								<td className="px-6 py-3">
									<span className="text-[11px] font-medium uppercase tracking-wide text-[#9ca3af]">Threshold </span>
									<span className="font-medium text-[#374151]">
										{change.thresholdAfter} of {change.rows.filter((r) => r.inAfter).length}
									</span>
								</td>
							</tr>
						</tbody>
					</table>
				</div>
			)}
		</div>
	)
}
