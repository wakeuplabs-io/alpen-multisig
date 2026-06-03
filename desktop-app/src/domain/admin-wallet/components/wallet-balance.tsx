import { useState } from 'react'
import { formatBtcFromSats } from '@/domain/admin-wallet/model/format-btc-from-sats'
import { formatUnconfirmedBalanceLine } from '@/domain/admin-wallet/model/format-unconfirmed-balance-line'

export type WalletBalanceProps = {
	confirmedSats: number
	unconfirmedSats: number
	isLoading: boolean
}

export function WalletBalance({ confirmedSats, unconfirmedSats, isLoading }: WalletBalanceProps) {
	const [showSats, setShowSats] = useState(false)

	if (isLoading) {
		return (
			<div className="rounded-2xl bg-[#f4f2ff] p-5">
				<div className="mb-3 h-9 w-44 animate-pulse rounded bg-[#e5e7eb]" />
				<div className="mb-2 h-4 w-28 animate-pulse rounded bg-[#e5e7eb]" />
				<div className="h-4 w-16 animate-pulse rounded bg-[#e5e7eb]" />
			</div>
		)
	}

	const btcStr = `${formatBtcFromSats(confirmedSats)} BTC`
	const satsStr = `${confirmedSats.toLocaleString()} sats`
	const primary = showSats ? satsStr : btcStr
	const secondary = showSats ? btcStr : satsStr
	const unconfirmedLine = formatUnconfirmedBalanceLine(unconfirmedSats)

	return (
		<div className="rounded-2xl bg-[#f4f2ff] p-5">
			<div className="flex items-baseline gap-0">
				<span className="font-['BIZ_UDPMincho'] text-[34px] font-normal leading-none text-[#111827]">
					{showSats ? confirmedSats.toLocaleString() : formatBtcFromSats(confirmedSats)}
				</span>
				<span className="ml-2 font-sans text-[13px] font-medium text-[#9480f5]">{showSats ? 'sats' : 'BTC'}</span>
			</div>
			<div className="mt-1.5 font-mono text-[12px] text-[#9ca3af]">{secondary}</div>
			{unconfirmedLine !== null && <div className="mt-1 font-mono text-[12px] text-[#6b7280]">{unconfirmedLine}</div>}
			<button
				type="button"
				onClick={() => setShowSats((prev) => !prev)}
				aria-pressed={showSats}
				className="mt-2 cursor-pointer bg-transparent p-0 text-[12px] text-[#9480f5] underline underline-offset-2 transition hover:text-[#7c6fcd]"
			>
				Show {showSats ? 'BTC' : 'sats'}
			</button>
			<span className="sr-only">
				Primary balance: {primary}
				{unconfirmedLine !== null ? `. Unconfirmed: ${unconfirmedLine}` : ''}
			</span>
		</div>
	)
}
