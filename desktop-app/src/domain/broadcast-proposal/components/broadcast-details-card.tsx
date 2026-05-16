import type { PrepareBroadcastResult } from '@/api/proposals'
import { satsToBtc } from '../model/broadcast-proposal'

type Props = {
	bundle: PrepareBroadcastResult
	onBroadcast: () => void
	isBroadcasting: boolean
}

export function BroadcastDetailsCard({ bundle, onBroadcast, isBroadcasting }: Props) {
	return (
		<div className="rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-sm">
			<h3 className="mb-4 text-base font-semibold text-[#111827]">Broadcast Details</h3>
			<dl className="space-y-3 text-sm">
				<div className="flex justify-between gap-4">
					<dt className="text-[#6b7280]">Commit address</dt>
					<dd className="break-all text-right font-mono text-xs text-[#111827]">{bundle.commitAddress}</dd>
				</div>
				<div className="flex justify-between gap-4">
					<dt className="text-[#6b7280]">Commit amount</dt>
					<dd className="font-medium text-[#111827]">
						{satsToBtc(bundle.commitAmountSats)} BTC ({bundle.commitAmountSats.toLocaleString()} sats)
					</dd>
				</div>
				<div className="flex justify-between gap-4">
					<dt className="text-[#6b7280]">Estimated fee</dt>
					<dd className="text-[#111827]">{bundle.estimatedFeeSats.toLocaleString()} sats</dd>
				</div>
			</dl>
			<button
				type="button"
				data-testid="e2e-broadcast-confirm"
				disabled={isBroadcasting}
				onClick={onBroadcast}
				className="mt-6 w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-sm font-medium text-white transition hover:bg-black disabled:cursor-not-allowed disabled:opacity-60"
			>
				{isBroadcasting ? 'Broadcasting…' : 'Confirm & Broadcast'}
			</button>
		</div>
	)
}
