import { CopyButton } from '@/components/copy-button'
import { SignaturePenMutedIcon } from '@/assets/icons'
import { SignerAdminId } from '@/components/signer-admin-id'
import { useAdminIds } from '@/hooks/use-admin-ids'
import { normalizeAdminId } from '@/lib/admin-id'
import { pendingSigners } from '@/lib/pending-signers'
import { normalizePubkey } from '@/lib/pubkey'

type Approval = { signerPubkey: string; signatureHex: string }

type Props = {
	signatures: Approval[]
	/** Every signer of the authority as the ASM lists it (Admin IDs), so the ones still missing can be listed. */
	allSigners: string[]
	/** The connected signer's key, highlighted with a YOU badge. */
	signerPubkey: string | null
	requiredSignatures: number
	/**
	 * True only while the proposal is still pending AND below quorum — missing signers read "Pending" then,
	 * "No signature" otherwise (#540). Quorum is checked too because the backend leaves a quorum-full proposal at
	 * `pending` until the desktop PATCHes it to `approved`; if that call fails, nothing is left for them to do either.
	 */
	isPending: boolean
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
	isPending,
}: Props) {
	// Signatures and the session carry keys; the role's set carries Admin IDs. Compared as Admin IDs.
	const adminIds = useAdminIds([...signatures.map((s) => s.signerPubkey), ...(signerPubkey ? [signerPubkey] : [])])
	const pending = pendingSigners(
		allSigners,
		signatures.map((s) => adminIds.get(normalizePubkey(s.signerPubkey))),
	)
	const myAdminId = signerPubkey ? adminIds.get(normalizePubkey(signerPubkey)) : undefined

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
							<SignerAdminId pubkey={sig.signerPubkey} className="flex-1 font-mono text-label text-[#111827]" />
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
					const isMe = !!myAdminId && normalizeAdminId(signer) === normalizeAdminId(myAdminId)
					return (
						<div key={`pending-${i}`} className="flex items-center gap-3 px-6 py-3">
							<span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full border-2 border-[#d1d5db] bg-white" />
							<SignerAdminId adminId={signer} className="flex-1 font-mono text-label text-[#6b7280]" />
							{isMe && (
								<span className="rounded-full border border-accent-border bg-bg-surface px-2 py-0.5 text-[10px] font-medium text-emphasis">
									YOU
								</span>
							)}
							<span className="shrink-0 text-label text-[#9ca3af]">{isPending ? 'Pending' : 'No signature'}</span>
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
