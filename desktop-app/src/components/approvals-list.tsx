import { CopyButton } from '@/components/copy-button'
import { SignaturePenMutedIcon } from '@/assets/icons'
import { truncatePubkey } from '@/lib/pubkey'

type Approval = { signerPubkey: string; signatureHex: string }

type Props = {
	signatures: Approval[]
	/** Every signer of the authority, so the ones still missing can be listed as Pending. */
	allSigners: string[]
	/** The connected signer, highlighted with a YOU badge. */
	signerPubkey: string | null
	requiredSignatures: number
	/** Section heading — 'Approvals' for a proposal, 'Cancel approvals' for a cancellation. */
	title?: string
}

/**
 * Who signed, who is still missing, and which row is me.
 *
 * Shared by the proposal screen and the cancel card: cancelling is the more consequential of the
 * two actions and used to show only a counter (#486).
 */
export function ApprovalsList({
	signatures,
	allSigners,
	signerPubkey,
	requiredSignatures,
	title = 'Approvals',
}: Props) {
	const pending = allSigners.filter(
		(signer) => !signatures.some((s) => s.signerPubkey.toLowerCase() === signer.toLowerCase()),
	)

	return (
		<div className="overflow-hidden rounded-xl border border-[#e5e7eb] bg-white shadow-sm">
			<div className="flex items-center justify-between border-b border-[#f3f4f6] px-6 py-4">
				<p className="m-0 text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]">
					{title} · {signatures.length} of {requiredSignatures}
				</p>
			</div>
			<div className="divide-y divide-[#f3f4f6]">
				{/* Signed rows */}
				{signatures.map((sig, i) => {
					const isMe = signerPubkey !== null && sig.signerPubkey.toLowerCase() === signerPubkey.toLowerCase()
					return (
						<div key={i} className="flex items-center gap-3 bg-[#f0fdf9] px-6 py-3">
							<span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-[#0f9d7a]">
								<SignaturePenMutedIcon width={10} height={10} style={{ filter: 'brightness(10)' }} />
							</span>
							<span className="flex-1 font-mono text-label text-[#111827]">{truncatePubkey(sig.signerPubkey)}</span>
							{isMe && (
								<span className="rounded-full border border-accent-border bg-bg-surface px-2 py-0.5 text-[10px] font-medium text-emphasis">
									YOU
								</span>
							)}
							<CopyButton text={sig.signatureHex} variant="icon" />
							<span className="shrink-0 text-label text-[#6b7280]">Signed</span>
						</div>
					)
				})}

				{/* Pending rows — signers of the authority not yet in signatures */}
				{pending.map((signer, i) => {
					const isMe = signerPubkey !== null && signer.toLowerCase() === signerPubkey.toLowerCase()
					return (
						<div key={`pending-${i}`} className="flex items-center gap-3 px-6 py-3">
							<span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full border-2 border-[#d1d5db] bg-white" />
							<span className="flex-1 font-mono text-label text-[#6b7280]">{truncatePubkey(signer)}</span>
							{isMe && (
								<span className="rounded-full border border-accent-border bg-bg-surface px-2 py-0.5 text-[10px] font-medium text-emphasis">
									YOU
								</span>
							)}
							<span className="shrink-0 text-label text-[#9ca3af]">Pending</span>
						</div>
					)
				})}

				{/* Fallback when the signer set could not be read and nobody has signed yet */}
				{allSigners.length === 0 && signatures.length === 0 && (
					<div className="px-6 py-4 text-body-sm text-[#9ca3af]">No signatures yet.</div>
				)}
			</div>
		</div>
	)
}
