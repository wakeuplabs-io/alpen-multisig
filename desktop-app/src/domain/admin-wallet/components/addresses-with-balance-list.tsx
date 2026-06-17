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
				className="flex w-full items-center justify-between px-0 py-2.5 text-left hover:bg-transparent"
				data-testid="e2e-wallet-addresses-toggle"
			>
				<span className="flex items-center font-sans text-body-sm font-medium text-[#374151]">
					Addresses with balance
					<span className="ml-2 rounded-full bg-[#f3f4f6] px-2 py-0.5 text-mono-sm tabular-nums text-[#6b7280]">
						{count}
					</span>
				</span>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					strokeWidth="2"
					strokeLinecap="round"
					strokeLinejoin="round"
					aria-hidden="true"
					className={`flex-none text-[#9ca3af] transition-transform duration-150 ${isExpanded ? 'rotate-180' : 'rotate-0'}`}
				>
					<polyline points="6 9 12 15 18 9" />
				</svg>
			</button>

			{isExpanded && (
				<div>
					{isLoading && rows === null && (
						<div className="space-y-1.5 py-3">
							{[0, 1, 2].map((i) => (
								<div key={i} className="h-4 w-full animate-pulse rounded bg-[#e5e7eb]" />
							))}
						</div>
					)}

					{error !== null && rows === null && !isLoading && (
						<div className="py-3 text-label text-[#ef4444]">{formatAdminWalletError(error).body}</div>
					)}

					{rows !== null && rows.length === 0 && !isLoading && error === null && (
						<div className="py-3 text-label text-[#9ca3af]" data-testid="e2e-wallet-addresses-empty">
							No addresses with balance yet
						</div>
					)}

					{rows !== null && rows.length > 0 && (
						<table className="w-full" data-testid="e2e-wallet-addresses-list">
							<thead>
								<tr className="text-mono-sm font-medium uppercase tracking-[0.08em] text-[#9ca3af]">
									<th className="py-1 pr-3 text-left font-medium">Address</th>
									<th className="px-3 py-1 text-right font-medium">Balance</th>
								</tr>
							</thead>
							<tbody className="divide-y divide-[#f3f4f6]">
								{rows.map((row) => (
									<AddressRow
										key={`${row.keychain}-${row.index}`}
										index={row.index}
										address={row.address}
										keychain={row.keychain}
										confirmedSats={row.confirmedSats}
										unconfirmedSats={row.unconfirmedSats}
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
