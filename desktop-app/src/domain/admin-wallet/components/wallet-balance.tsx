import { useState } from 'react'
import {
	denominateSats,
	formatDenominatedBalance,
	toggleDenomination,
	type Denomination,
} from '@/domain/admin-wallet/model/balance-denomination'
import { formatInFlightBalanceLine } from '@/domain/admin-wallet/model/format-in-flight-balance-line'
import { formatUnconfirmedBalanceLine } from '@/domain/admin-wallet/model/format-unconfirmed-balance-line'

export type WalletBalanceProps = {
	confirmedSats: number
	unconfirmedSats: number
	/** Coins held by in-flight transactions — shown as its own line, never in the balance. */
	reservedSats: number
	isLoading: boolean
}

export function WalletBalance({ confirmedSats, unconfirmedSats, reservedSats, isLoading }: WalletBalanceProps) {
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
	const inFlightLine = formatInFlightBalanceLine(reservedSats)

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
					className="ml-2 cursor-pointer bg-transparent p-0 font-sans text-body-sm font-medium text-emphasis-soft underline decoration-dotted underline-offset-4 transition hover:text-emphasis"
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
			{inFlightLine !== null && (
				<div
					className="mt-1.5 flex items-center gap-1.5 font-mono text-label text-[#6b7280]"
					data-testid="e2e-wallet-balance-in-flight"
				>
					<span className="h-1.5 w-1.5 flex-none rounded-full bg-[#9ca3af]" aria-hidden="true" />
					{inFlightLine}
				</div>
			)}
			<span className="sr-only">
				Primary balance: {formatDenominatedBalance(shown)}
				{unconfirmedLine !== null ? `. Unconfirmed: ${unconfirmedLine}` : ''}
				{inFlightLine !== null ? `. In flight: ${inFlightLine}` : ''}
			</span>
		</div>
	)
}
