import type { AdminWalletError } from '@/domain/admin-wallet/model/types'
import type { AddressWithBalanceView } from '@/domain/admin-wallet/model/view-models'
import { formatAdminWalletError } from '@/domain/admin-wallet/model/format-admin-wallet-error'
import { AddressRow } from './address-row'

export type AddressesWithBalanceListProps = {
	rows: AddressWithBalanceView[] | null
	isLoading: boolean
	error: AdminWalletError | null
	isExpanded: boolean
	onToggle(): void
}

export function AddressesWithBalanceList({
	rows,
	isLoading,
	error,
	isExpanded,
	onToggle,
}: AddressesWithBalanceListProps) {
	const count = rows?.length ?? 0

	return (
		<div>
			<button
				type="button"
				onClick={onToggle}
				className="flex w-full items-center justify-between px-4 py-2.5 text-left text-[13px] font-medium text-[#374151] hover:bg-[#f9fafb]"
			>
				<span>All addresses with balance · {count}</span>
				<span className="text-[#9ca3af]">{isExpanded ? '▲' : '▼'}</span>
			</button>

			{isExpanded && (
				<div>
					{isLoading && rows === null && (
						<div className="space-y-1.5 px-4 py-3">
							{[0, 1, 2].map((i) => (
								<div key={i} className="h-4 w-full animate-pulse rounded bg-[#e5e7eb]" />
							))}
						</div>
					)}

					{error !== null && rows === null && !isLoading && (
						<div className="px-4 py-3 text-[12px] text-[#ef4444]">{formatAdminWalletError(error).body}</div>
					)}

					{rows !== null && rows.length === 0 && !isLoading && error === null && (
						<div className="px-4 py-3 text-[12px] text-[#9ca3af]">No addresses with balance yet</div>
					)}

					{rows !== null && rows.length > 0 && (
						<table className="w-full">
							<thead>
								<tr className="text-[11px] text-[#9ca3af]">
									<th className="py-1 pl-4 pr-3 text-left font-medium">#</th>
									<th className="px-3 py-1 text-left font-medium">Address</th>
									<th className="px-3 py-1 text-right font-medium">Balance</th>
								</tr>
							</thead>
							<tbody>
								{rows.map((row) => (
									<AddressRow
										key={row.index}
										index={row.index}
										address={row.address}
										balanceSats={row.balanceSats}
										isUsed={row.isUsed}
									/>
								))}
							</tbody>
						</table>
					)}
				</div>
			)}
		</div>
	)
}
