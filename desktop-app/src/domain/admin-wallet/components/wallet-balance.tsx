import { useState } from 'react'
import {
	denominateSats,
	formatDenominatedBalance,
	toggleDenomination,
	type Denomination,
} from '@/domain/admin-wallet/model/balance-denomination'
import { formatUnconfirmedBalanceLine } from '@/domain/admin-wallet/model/format-unconfirmed-balance-line'

export type WalletBalanceProps = {
	confirmedSats: number
	unconfirmedSats: number
	isLoading: boolean
}

export function WalletBalance({ confirmedSats, unconfirmedSats, isLoading }: WalletBalanceProps) {
	const [denomination, setDenomination] = useState<Denomination>('BTC')

	if (isLoading) {
		return (
			<div className="rounded-2xl bg-bg-surface px-5 py-6">
				<div className="mb-3 h-9 w-44 animate-pulse rounded bg-[#e5e7eb]" />
				<div className="mb-2 h-4 w-28 animate-pulse rounded bg-[#e5e7eb]" />
				<div className="h-4 w-16 animate-pulse rounded bg-[#e5e7eb]" />
			</div>
		)
	}

	const alternateDenomination = toggleDenomination(denomination)
	const shown = denominateSats(confirmedSats, denomination)
	const alternate = denominateSats(confirmedSats, alternateDenomination)
	const unconfirmedLine = formatUnconfirmedBalanceLine(unconfirmedSats)

	return (
		<div className="rounded-2xl bg-bg-surface px-5 py-6">
			<div className="flex items-baseline gap-0">
				<span
					className="font-display text-[34px] font-normal leading-none text-[#111827]"
					data-testid="e2e-wallet-balance-primary"
				>
					{shown.amount}
				</span>
				<button
					type="button"
					onClick={() => setDenomination(alternateDenomination)}
					title={`Show balance in ${alternateDenomination}`}
					aria-label={`Balance shown in ${shown.unit}. Show in ${alternateDenomination}`}
					className="ml-2 cursor-pointer bg-transparent p-0 font-sans text-body-sm font-medium text-accent underline decoration-dotted underline-offset-4 transition hover:text-accent-hover"
					data-testid="e2e-wallet-balance-unit"
				>
					{shown.unit}
				</button>
			</div>
			<div className="mt-2 font-mono text-label text-[#9ca3af]" data-testid="e2e-wallet-balance-secondary">
				{formatDenominatedBalance(alternate)}
			</div>
			{unconfirmedLine !== null && (
				<div
					className="mt-1.5 flex items-center gap-1.5 font-mono text-label text-[#6b7280]"
					data-testid="e2e-wallet-balance-unconfirmed"
				>
					<span className="h-1.5 w-1.5 flex-none rounded-full bg-emphasis-soft" aria-hidden="true" />
					{unconfirmedLine}
				</div>
			)}
			<span className="sr-only">
				Primary balance: {formatDenominatedBalance(shown)}
				{unconfirmedLine !== null ? `. Unconfirmed: ${unconfirmedLine}` : ''}
			</span>
		</div>
	)
}
