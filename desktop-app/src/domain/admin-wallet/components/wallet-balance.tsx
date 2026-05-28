import { useState } from 'react'
import { formatBtcFromSats } from '@/domain/admin-wallet/model/format-btc-from-sats'

export type WalletBalanceProps = {
	balanceSats: number
	isLoading: boolean
}

export function WalletBalance({ balanceSats, isLoading }: WalletBalanceProps) {
	const [showSats, setShowSats] = useState(false)

	if (isLoading) {
		return (
			<div className="px-5 py-4">
				<div className="mb-1 h-7 w-32 animate-pulse rounded bg-[#e5e7eb]" />
				<div className="h-4 w-20 animate-pulse rounded bg-[#e5e7eb]" />
			</div>
		)
	}

	const primaryValue = showSats ? `${balanceSats.toLocaleString()} sats` : `${formatBtcFromSats(balanceSats)} BTC`

	const secondaryValue = showSats ? `${formatBtcFromSats(balanceSats)} BTC` : `${balanceSats.toLocaleString()} sats`

	return (
		<div className="px-5 py-4">
			<p className="text-[22px] font-semibold leading-tight text-[#111827]">{primaryValue}</p>
			<div className="mt-1 flex items-center gap-2">
				<p className="text-[13px] text-[#6b7280]">{secondaryValue}</p>
				<button
					type="button"
					onClick={() => setShowSats((prev) => !prev)}
					className="rounded px-1.5 py-0.5 text-[11px] font-medium text-[#6366f1] transition hover:bg-[#eef2ff]"
				>
					{showSats ? 'Show BTC' : 'Show sats'}
				</button>
			</div>
		</div>
	)
}
