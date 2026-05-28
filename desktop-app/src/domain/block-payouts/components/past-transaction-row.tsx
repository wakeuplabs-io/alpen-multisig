import type { PastBlockPayoutTx } from '../model/block-payouts.types'

type Props = {
	tx: PastBlockPayoutTx
	onRebroadcast: () => void
	onCopyRawTx: () => void
}

function truncate(str: string, head = 10, tail = 8): string {
	if (str.length <= head + tail + 3) return str
	return `${str.slice(0, head)}…${str.slice(-tail)}`
}

function formatDate(d: Date): string {
	return d.toLocaleString(undefined, {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
		hour: '2-digit',
		minute: '2-digit',
	})
}

export function PastTransactionRow({ tx, onRebroadcast, onCopyRawTx }: Props) {
	const isConfirmed = tx.status === 'confirmed'

	return (
		<div className="flex items-center gap-4 rounded-xl border border-[#e5e7eb] bg-white px-5 py-3.5">
			{/* TX ID */}
			<div className="min-w-0 flex-1">
				<span className="font-mono text-[13px] font-medium text-[#0a0a0a]">{truncate(tx.id, 12, 6)}</span>
			</div>

			{/* Status badge */}
			<span
				className="inline-flex shrink-0 items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-medium"
				style={
					isConfirmed
						? { background: '#ecfdf5', color: '#059669', borderColor: '#a7f3d0' }
						: { background: '#eff6ff', color: '#2563eb', borderColor: '#bfdbfe' }
				}
			>
				<span
					className="h-1.5 w-1.5 flex-none rounded-full"
					style={{ background: isConfirmed ? '#059669' : '#2563eb' }}
					aria-hidden="true"
				/>
				{isConfirmed ? 'Confirmed' : 'Unconfirmed'}
			</span>

			{/* Timestamp */}
			<span className="w-44 shrink-0 text-right text-[12px] text-[#6b7280]">
				{isConfirmed && tx.blockTimestamp ? formatDate(tx.blockTimestamp) : formatDate(tx.broadcastAt)}
			</span>

			{/* Actions */}
			<div className="flex shrink-0 items-center gap-1.5">
				{!isConfirmed && (
					<>
						<button
							type="button"
							className="inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-3 py-1.5 text-[12px] font-medium text-[#374151] transition hover:border-[#d1d5db] hover:bg-[#f9fafb]"
							onClick={onRebroadcast}
						>
							<svg width={12} height={12} viewBox="0 0 24 24" fill="none" aria-hidden>
								<polyline
									points="1 4 1 10 7 10"
									stroke="currentColor"
									strokeWidth="1.5"
									strokeLinecap="round"
									strokeLinejoin="round"
								/>
								<path d="M3.51 15a9 9 0 1 0 .49-4.98" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
							</svg>
							Rebroadcast
						</button>
						<button
							type="button"
							title="Copy raw transaction"
							aria-label="Copy raw transaction"
							className="flex h-7 w-7 items-center justify-center rounded-md border border-[#e5e7eb] bg-white text-[#6b7280] transition hover:border-[#d1d5db] hover:bg-[#f9fafb] hover:text-[#374151]"
							onClick={onCopyRawTx}
						>
							<svg width={13} height={13} viewBox="0 0 24 24" fill="none" aria-hidden>
								<rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" strokeWidth="1.5" />
								<path
									d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
									stroke="currentColor"
									strokeWidth="1.5"
								/>
							</svg>
						</button>
					</>
				)}
			</div>
		</div>
	)
}
