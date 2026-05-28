type TxHistoryItemProps = {
	txid: string
	amountSats: number
	confirmations: number
	timestamp?: string
}

export function TxHistoryItem({ txid, amountSats, confirmations, timestamp }: TxHistoryItemProps) {
	return (
		<div className="flex items-center justify-between border-b border-[#f3f4f6] px-4 py-3 last:border-0">
			<div className="min-w-0 flex-1">
				<p className="truncate font-mono text-[12px] text-[#111827]">{txid}</p>
				{timestamp && <p className="mt-0.5 text-[11px] text-[#9ca3af]">{timestamp}</p>}
			</div>
			<div className="ml-4 text-right">
				<p className="text-[13px] font-medium text-[#111827]">{amountSats.toLocaleString()} sats</p>
				<p className="text-[11px] text-[#9ca3af]">{confirmations} conf</p>
			</div>
		</div>
	)
}
