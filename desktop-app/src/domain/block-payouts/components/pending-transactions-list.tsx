import type { PendingBlockPayoutTx } from '../model/block-payouts.types'
import { PendingTransactionCard } from './pending-transaction-card'

type Props = {
	txs: PendingBlockPayoutTx[]
	hasConflicts: boolean
	onSign: (txId: string) => void
	onPasteSignatures: (txId: string) => void
	onExport: (txId: string) => void
	onCopySignatures: (txId: string) => void
}

export function PendingTransactionsList({
	txs,
	hasConflicts,
	onSign,
	onPasteSignatures,
	onExport,
	onCopySignatures,
}: Props) {
	if (txs.length === 0) {
		return (
			<div className="rounded-xl border border-[#e5e7eb] bg-white px-6 py-12 text-center">
				<div className="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-full border border-[#e5e7eb] bg-[#f8f8fb] text-[#9ca3af]">
					<svg width={18} height={18} viewBox="0 0 24 24" fill="none" aria-hidden>
						<circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="1.5" />
						<path d="M12 7v5l3 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
					</svg>
				</div>
				<p className="m-0 font-['BIZ_UDPMincho'] text-[18px] text-[#0a0a0a]">No pending transactions</p>
				<p className="m-0 mt-1.5 text-[13px] text-[#6b7280]">
					Create a block payouts transaction to start collecting signatures.
				</p>
			</div>
		)
	}

	return (
		<div className="flex flex-col gap-3">
			{hasConflicts && (
				<div className="flex items-start gap-2.5 rounded-xl border border-[#fde68a] bg-[#fffbeb] px-4 py-3">
					<svg
						width={16}
						height={16}
						viewBox="0 0 24 24"
						fill="none"
						aria-hidden
						className="mt-0.5 shrink-0 text-[#d97706]"
					>
						<circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="1.5" />
						<path d="M12 8v4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
						<circle cx="12" cy="16" r="1" fill="currentColor" />
					</svg>
					<p className="m-0 text-[13px] text-[#92400e]">
						Some pending transactions share inputs. See the <span className="font-medium">ⓘ icons</span> for details.
					</p>
				</div>
			)}
			{txs.map((tx) => (
				<PendingTransactionCard
					key={tx.id}
					tx={tx}
					onSign={() => onSign(tx.id)}
					onPasteSignatures={() => onPasteSignatures(tx.id)}
					onExport={() => onExport(tx.id)}
					onCopySignatures={() => onCopySignatures(tx.id)}
				/>
			))}
		</div>
	)
}
