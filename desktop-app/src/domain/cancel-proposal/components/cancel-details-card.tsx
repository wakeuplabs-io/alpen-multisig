import { CopyButton } from '@/components/copy-button'
import { ApprovalsList } from '@/components/approvals-list'
import { CheckCircleEmeraldIcon } from '@/assets/icons'
import type { CancelProposalSummary } from '@/api/proposals'
import { deviceCopy } from '@/lib/device-copy'
import { truncatePubkey } from '@/lib/pubkey'
import type { WalletVendor } from '@/wallet/types'

type Props = {
	cancelProposal: CancelProposalSummary
	/** The cancellation's own sequence number — null while its proposal row is still loading. */
	cancelSeqNo: number | null
	/** Reviewable cancellation payload — null while its proposal row is still loading. */
	cancelActionHex: string | null
	/** Whether the fetch backing the two fields above is still in flight. */
	isLoadingDetails: boolean
	/** Action id of the update being cancelled. */
	targetActionId: string
	/** ASM queue update id the CancelAction targets — null until the target's reveal confirmed. */
	targetUpdateId: number | null
	/** Every signer of the authority, so the ones still missing are listed as Pending. */
	allSigners: string[]
	signerPubkey: string | null
	/** Vendor actually connected, so the sign button never names a device the signer lacks (#487). */
	walletVendor: WalletVendor
	onSign: () => void
	onBroadcast: () => void
}

export function CancelDetailsCard({
	cancelProposal,
	cancelSeqNo,
	cancelActionHex,
	isLoadingDetails,
	targetActionId,
	targetUpdateId,
	allSigners,
	signerPubkey,
	walletVendor,
	onSign,
	onBroadcast,
}: Props) {
	const collected = cancelProposal.signatures.length
	const required = cancelProposal.requiredSignatures
	const progress = required === 0 ? 100 : Math.min((collected / required) * 100, 100)
	const hasQuorum = collected >= required
	const alreadySigned =
		signerPubkey !== null &&
		cancelProposal.signatures.some((s) => s.signerPubkey.toLowerCase() === signerPubkey.toLowerCase())

	const sigsJson = JSON.stringify(cancelProposal.signatures, null, 2)

	return (
		<div className="space-y-4">
			<div className="overflow-hidden rounded-xl border border-accent-border bg-highlight-surface shadow-sm">
				<div className="border-b border-accent-border px-6 py-4">
					<p className="m-0 text-mono-sm font-semibold uppercase tracking-wider text-emphasis">Cancel proposal</p>
				</div>
				<div className="px-6 py-5 space-y-4">
					{/* Identity — which cancellation this is, and which update it removes */}
					<div>
						{isLoadingDetails ? (
							<div className="h-3 w-56 animate-pulse rounded bg-[#e5e7eb]" />
						) : (
							<p className="m-0 text-body-sm text-[#374151]">
								{cancelSeqNo !== null && <>Cancel #{cancelSeqNo} · </>}
								Cancels <span className="font-mono">{truncatePubkey(targetActionId)}</span>
								{targetUpdateId !== null && <> · Queue update ID {targetUpdateId}</>}
							</p>
						)}
					</div>

					{/* Reviewable cancellation payload */}
					{isLoadingDetails ? (
						<div className="h-12 animate-pulse rounded-lg bg-[#e5e7eb]" />
					) : (
						cancelActionHex !== null && (
							<div>
								<div className="mb-1.5 flex items-center justify-between gap-3">
									<p className="m-0 text-body-sm font-medium text-[#111827]">Cancel payload</p>
									<CopyButton text={cancelActionHex} label="Copy payload" />
								</div>
								<p className="m-0 break-all rounded-lg border border-[#e5e7eb] bg-white px-3 py-2.5 font-mono text-mono-sm leading-relaxed text-[#374151]">
									{cancelActionHex}
								</p>
							</div>
						)
					)}

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

					{/*
					 * Own participation, stated independently of quorum: this note used to be the
					 * `else` branch of the quorum ternary, so it vanished exactly when the signer was
					 * about to broadcast an irreversible action (#486).
					 */}
					{alreadySigned && (
						<div className="rounded-xl border border-[#d1fae5] bg-[#f0fdf4] px-4 py-3">
							<p className="m-0 text-body-sm font-medium text-[#065f46]">
								You have signed this cancellation.
								{!hasQuorum && ' Waiting for other signers to reach quorum.'}
							</p>
						</div>
					)}

					{hasQuorum && (
						<p className="m-0 inline-flex items-center gap-1.5 text-body-sm font-medium text-[#0f9d7a]">
							<CheckCircleEmeraldIcon width={14} height={14} className="block shrink-0" />
							Quorum reached — ready to send
						</p>
					)}

					{/* Action buttons */}
					{hasQuorum ? (
						<button
							type="button"
							className="w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-body font-medium text-white transition hover:bg-black"
							onClick={onBroadcast}
						>
							Send cancel tx
						</button>
					) : (
						!alreadySigned && (
							<button
								type="button"
								className="w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-body font-medium text-white transition hover:bg-black"
								onClick={onSign}
							>
								Sign with {deviceCopy(walletVendor).label}
							</button>
						)
					)}
				</div>
			</div>

			<ApprovalsList
				signatures={cancelProposal.signatures}
				allSigners={allSigners}
				signerPubkey={signerPubkey}
				requiredSignatures={required}
				title="Cancel approvals"
			/>
		</div>
	)
}
