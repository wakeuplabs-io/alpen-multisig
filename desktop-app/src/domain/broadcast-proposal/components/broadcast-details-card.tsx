import { useState } from 'react'
import type { PrepareBroadcastResult, Proposal } from '@/api/proposals'
import { CopyClipboardIcon } from '@/assets/icons'
import { satsToBtc } from '../model/broadcast-proposal'

type Props = {
	bundle: PrepareBroadcastResult
	proposal: Proposal | null
	onBroadcast: () => void
	isBroadcasting: boolean
}

function CopyButton({ text }: { text: string }) {
	const [copied, setCopied] = useState(false)

	function handleCopy() {
		void navigator.clipboard.writeText(text).then(() => {
			setCopied(true)
			setTimeout(() => setCopied(false), 2000)
		})
	}

	return (
		<button
			type="button"
			onClick={handleCopy}
			className="inline-flex shrink-0 items-center gap-1 rounded-md border border-[#e5e7eb] bg-white px-2.5 py-1.5 text-xs font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
		>
			<CopyClipboardIcon width={12} height={12} />
			{copied ? 'Copied!' : 'Copy'}
		</button>
	)
}

function SectionLabel({ children }: { children: string }) {
	return <p className="mb-2 text-[11px] font-semibold uppercase tracking-wider text-[#9ca3af]">{children}</p>
}

export function BroadcastDetailsCard({ bundle, proposal, onBroadcast, isBroadcasting }: Props) {
	const collectedSignatures = proposal?.signatures.length ?? 0
	const requiredSignatures = proposal?.requiredSignatures ?? 0
	const signaturesProgress =
		requiredSignatures === 0 ? 100 : Math.min((collectedSignatures / requiredSignatures) * 100, 100)

	return (
		<div className="overflow-hidden rounded-xl border border-[#e5e7eb] bg-white shadow-sm">
			{proposal && (
				<div className="border-b border-[#f3f4f6] p-6 pb-5">
					<div className="flex items-start justify-between gap-3">
						<div className="min-w-0 flex-1">
							<h2 className="m-0 font-['BIZ_UDPMincho'] text-[26px] leading-[1.2] text-[#0a0a0a]">
								Proposal #{proposal.seqNo}
							</h2>
							<p className="m-0 mt-1 text-[13px] text-[#6b7280]">{proposal.authority}</p>
						</div>
						<span className="inline-flex shrink-0 items-center gap-1.5 rounded-md border border-[#a7f3d0] bg-[#ecfdf5] px-2.5 py-0.75 text-[11px] font-medium whitespace-nowrap text-[#059669]">
							<span className="h-1.5 w-1.5 flex-none rounded-full bg-[#059669]" aria-hidden="true" />
							Quorum reached
						</span>
					</div>

					<div className="mt-4">
						<div className="mb-1.5 flex items-center justify-between gap-3">
							<p className="m-0 text-[13px] font-medium text-[#121212]">Signatures</p>
							<p className="m-0 text-[13px] font-medium text-[#121212]">
								{collectedSignatures} / {requiredSignatures} <span className="font-normal text-[#6b7280]">signed</span>
							</p>
						</div>
						<div className="h-1.5 rounded-full bg-[#ebedf0]">
							<div
								className="h-1.5 rounded-full transition-all"
								style={{ width: `${signaturesProgress}%`, background: '#0f9d7a' }}
							/>
						</div>
					</div>
				</div>
			)}

			<div className="space-y-5 p-6">
				<div>
					<SectionLabel>Commit TX</SectionLabel>
					<div className="flex items-start gap-2 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
						<span className="min-w-0 flex-1 break-all font-mono text-[12px] leading-relaxed text-[#111827]">
							{bundle.commitAddress}
						</span>
						<CopyButton text={bundle.commitAddress} />
					</div>
					<p className="mt-2 text-[13px] text-[#6b7280]">
						{satsToBtc(bundle.commitAmountSats)} BTC{' '}
						<span className="text-[12px] text-[#9ca3af]">({bundle.commitAmountSats.toLocaleString()} sats)</span>
					</p>
				</div>

				<div>
					<SectionLabel>Reveal TX</SectionLabel>
					<p className="text-[13px] text-[#6b7280]">
						Broadcast automatically once the commit transaction confirms on-chain.
					</p>
				</div>

				<div className="flex items-center justify-between rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
					<span className="text-[11px] font-semibold uppercase tracking-wider text-[#9ca3af]">Estimated fee</span>
					<span className="text-[13px] font-medium text-[#111827]">
						{bundle.estimatedFeeSats.toLocaleString()} sats
					</span>
				</div>

				<button
					type="button"
					data-testid="e2e-broadcast-confirm"
					disabled={isBroadcasting}
					onClick={onBroadcast}
					className="w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-sm font-medium text-white transition hover:bg-black disabled:cursor-not-allowed disabled:opacity-60"
				>
					{isBroadcasting ? 'Broadcasting…' : 'Confirm & Broadcast'}
				</button>
			</div>
		</div>
	)
}
