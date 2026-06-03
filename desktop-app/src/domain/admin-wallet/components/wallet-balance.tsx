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
			<div className="px-[18px] pt-[18px]">
				<div className="mb-2 h-8 w-44 animate-pulse rounded bg-[#e5e7eb]" />
				<div className="h-4 w-24 animate-pulse rounded bg-[#e5e7eb]" />
			</div>
		)
	}

	const btcStr = `${formatBtcFromSats(confirmedSats)} BTC`
	const satsStr = `${confirmedSats.toLocaleString()} sats`
	const primary = showSats ? satsStr : btcStr
	const secondary = showSats ? btcStr : satsStr
	const unconfirmedLine = formatUnconfirmedBalanceLine(unconfirmedSats)

	return (
		<div className="flex-none px-[18px] pt-[18px]">
			<div className="flex items-baseline gap-2 flex-wrap">
				<span className="font-['BIZ_UDPMincho'] text-[28px] font-normal leading-[1.2] text-[#111827]">
					{showSats ? confirmedSats.toLocaleString() : formatBtcFromSats(confirmedSats)}
				</span>
				<span className="text-[13px] font-medium text-[#6b7280]">{showSats ? 'sats' : 'BTC'}</span>
			</div>
			<button
				type="button"
				onClick={() => setShowSats((prev) => !prev)}
				aria-pressed={showSats}
				className="mt-2 cursor-pointer bg-transparent p-0 text-[12px] text-[#7c6fcd] underline underline-offset-2 transition hover:text-[#5a4fb3]"
			>
				Show {showSats ? 'BTC' : 'sats'}
			</button>
			<div className="mt-1.5 font-mono text-[12px] text-[#9ca3af]">{secondary}</div>
			{unconfirmedLine !== null && <div className="mt-1 font-mono text-[12px] text-[#6b7280]">{unconfirmedLine}</div>}
			<span className="sr-only">
				Primary balance: {primary}
				{unconfirmedLine !== null ? `. Unconfirmed: ${unconfirmedLine}` : ''}
			</span>
		</div>
	)
}
