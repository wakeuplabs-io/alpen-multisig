import { CopyButton } from '@/components/copy-button'
import type { SendResultDto } from '@/domain/admin-wallet/model/types'
import { formatBtcFromSats } from '@/domain/admin-wallet/model/format-btc-from-sats'
import { truncTxid } from '@/domain/admin-wallet/model/trunc-txid'

export type SendResultCardProps = {
	result: SendResultDto
	onDone(): void
}

/** Success surface for a broadcast send (PRD §4.3.5.5.1 — show the txid). */
export function SendResultCard({ result, onDone }: SendResultCardProps) {
	return (
		<div className="rounded-xl border border-[#a7f3d0] bg-[#ecfdf5] px-3 py-2.5" data-testid="e2e-wallet-send-success">
			<p className="m-0 text-[12px] font-medium text-[#047857]">
				{result.drained ? 'Sent — wallet drained' : 'Transaction broadcast'}
			</p>
			<p className="m-0 mt-0.5 text-[11px] text-[#065f46]">
				{result.amountSats.toLocaleString()} sats ({formatBtcFromSats(result.amountSats)} BTC) · fee{' '}
				{result.feeSats.toLocaleString()} sats
			</p>
			<p
				className="m-0 mt-1 flex items-center gap-1.5 font-mono text-[12px] text-[#065f46]"
				title={result.txid}
				data-testid="e2e-wallet-send-txid"
			>
				{truncTxid(result.txid)}
				<CopyButton text={result.txid} variant="icon" />
			</p>
			<button
				type="button"
				onClick={onDone}
				className="mt-2 rounded-lg border border-[#a7f3d0] bg-white px-2.5 py-1 text-[11px] font-medium text-[#047857] transition hover:border-[#047857]"
			>
				Done
			</button>
		</div>
	)
}
