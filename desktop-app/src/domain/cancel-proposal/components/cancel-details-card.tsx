import { CopyButton } from '@/components/copy-button'
import type { CancelProposalSummary } from '@/api/proposals'
import { deviceCopy } from '@/lib/device-copy'
import type { WalletVendor } from '@/wallet/types'

type Props = {
	cancelProposal: CancelProposalSummary
	signerPubkey: string | null
	/** Vendor actually connected, so the sign button never names a device the signer lacks (#487). */
	walletVendor: WalletVendor
	onSign: () => void
	onBroadcast: () => void
}

export function CancelDetailsCard({ cancelProposal, signerPubkey, walletVendor, onSign, onBroadcast }: Props) {
	const collected = cancelProposal.signatures.length
	const required = cancelProposal.requiredSignatures
	const progress = required === 0 ? 100 : Math.min((collected / required) * 100, 100)
	const hasQuorum = collected >= required
	const alreadySigned =
		signerPubkey !== null &&
		cancelProposal.signatures.some((s) => s.signerPubkey.toLowerCase() === signerPubkey.toLowerCase())

	const sigsJson = JSON.stringify(cancelProposal.signatures, null, 2)

	return (
		<div className="overflow-hidden rounded-xl border border-accent-border bg-highlight-surface shadow-sm">
			<div className="border-b border-accent-border px-6 py-4">
				<p className="m-0 text-mono-sm font-semibold uppercase tracking-wider text-emphasis">Cancel proposal</p>
			</div>
			<div className="px-6 py-5 space-y-4">
				{/* Sig progress */}
				<div>
					<div className="mb-1.5 flex items-center justify-between gap-3">
						<p className="m-0 text-body-sm font-medium text-[#111827]">Cancel signatures</p>
						<div className="flex items-center gap-2">
							<span className="text-body-sm font-medium text-[#111827]">
								{collected} / {required} <span className="font-normal text-[#6b7280]">signed</span>
							</span>
							{cancelProposal.signatures.length > 0 && <CopyButton text={sigsJson} label="Copy sigs" />}
						</div>
					</div>
					<div className="h-1.5 rounded-full bg-[#e5e7eb]">
						<div
							className="h-1.5 rounded-full transition-all"
							style={{ width: `${progress}%`, background: hasQuorum ? '#0f9d7a' : '#111827' }}
						/>
					</div>
				</div>

				{/* Action buttons */}
				{hasQuorum ? (
					<button
						type="button"
						className="w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-body font-medium text-white transition hover:bg-black"
						onClick={onBroadcast}
					>
						Send cancel tx
					</button>
				) : alreadySigned ? (
					<div className="rounded-xl border border-[#d1fae5] bg-[#f0fdf4] px-4 py-3">
						<p className="m-0 text-body-sm font-medium text-[#065f46]">
							You have signed. Waiting for other signers to reach quorum.
						</p>
					</div>
				) : (
					<button
						type="button"
						className="w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-body font-medium text-white transition hover:bg-black"
						onClick={onSign}
					>
						Sign with {deviceCopy(walletVendor).label}
					</button>
				)}
			</div>
		</div>
	)
}
