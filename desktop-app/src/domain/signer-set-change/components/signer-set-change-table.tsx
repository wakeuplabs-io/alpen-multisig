import type { SignerSetChange } from '../model/build-signer-set-change'

type Props = {
	change: SignerSetChange
}

export function SignerSetChangeTable({ change }: Props) {
	const signerCountBefore = change.rows.filter((row) => row.inBefore).length
	const signerCountAfter = change.rows.filter((row) => row.inAfter).length

	return (
		<table
			className="w-full table-fixed border-collapse"
			data-testid="e2e-signer-set-change-table"
			aria-label="Signer set change"
		>
			<thead>
				<tr className="border-b border-[#f3f4f6] bg-[#f9fafb]">
					<th
						scope="col"
						className="w-1/2 px-4 py-2.5 text-left text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]"
					>
						Before
					</th>
					<th
						scope="col"
						className="w-1/2 border-l border-[#f3f4f6] px-4 py-2.5 text-left text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]"
					>
						After
					</th>
				</tr>
			</thead>
			<tbody>
				{change.rows.map((row) => (
					<tr key={row.pubkey} className="border-b border-[#f3f4f6] last:border-0">
						<td className="px-4 py-2.5 align-top">
							{row.inBefore ? (
								<span
									className={`break-all font-mono text-mono-sm leading-relaxed ${
										row.isRemoved ? 'text-emphasis-soft line-through' : 'text-[#374151]'
									}`}
									aria-label={row.isRemoved ? `Removed signer ${row.pubkey}` : undefined}
								>
									{row.isRemoved && (
										<span className="mr-1 text-emphasis-soft" aria-hidden="true">
											−
										</span>
									)}
									{row.pubkey}
								</span>
							) : (
								<span className="text-[#9ca3af]" aria-label="Not present before">
									—
								</span>
							)}
						</td>
						<td className="border-l border-[#f3f4f6] px-4 py-2.5 align-top">
							{row.inAfter ? (
								<span
									className={`break-all font-mono text-mono-sm leading-relaxed ${
										row.isAdded ? 'font-medium text-[#0f9d7a]' : 'text-[#374151]'
									}`}
									aria-label={row.isAdded ? `Added signer ${row.pubkey}` : undefined}
								>
									{row.isAdded && (
										<span className="mr-1 text-[#0f9d7a]" aria-hidden="true">
											+
										</span>
									)}
									{row.pubkey}
								</span>
							) : (
								<span className="text-[#9ca3af]" aria-label="Not present after">
									—
								</span>
							)}
						</td>
					</tr>
				))}
				<tr className="border-t border-[#e5e7eb] bg-[#f9fafb]">
					<td className="px-4 py-2.5 text-label">
						<span className="text-[#9ca3af]">Threshold </span>
						{change.thresholdBefore !== null ? (
							<span className="font-mono font-medium text-[#374151]">
								{change.thresholdBefore} of {signerCountBefore}
							</span>
						) : (
							<span className="text-[#9ca3af]">—</span>
						)}
					</td>
					<td className="border-l border-[#f3f4f6] px-4 py-2.5 text-label">
						<span className="text-[#9ca3af]">Threshold </span>
						<span className="font-mono font-medium text-[#374151]">
							{change.thresholdAfter} of {signerCountAfter}
						</span>
					</td>
				</tr>
			</tbody>
		</table>
	)
}
