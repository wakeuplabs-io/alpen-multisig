import { useState } from 'react'
import type { Proposal, ProposalStatus } from '@/api/proposals'
import { saveJsonFile, writeClipboard } from '@/api/tauri-bridge'
import {
	CheckCircleEmeraldIcon,
	CopyClipboardIcon,
	DownloadIcon,
	ImportJsonIcon,
	SendIcon,
	SignaturePenMutedIcon,
} from '@/assets/icons'
import { ImportBundleModal, type ImportBroadcastState } from '@/domain/proposal-detail/components/import-bundle-modal'
import type { DecodedProposalData } from '@/domain/proposal-detail/hooks/use-decoded-proposal'
import type { PastedSignature } from '@/domain/proposal-detail/model/pasted-signature'

import { deriveProposalActions } from '@/domain/proposal-detail/model/derive-proposal-actions'
import { inferProposalTypeLabel } from '@/lib/proposal-type-label'

type Props = {
	proposal: Proposal
	signerPubkey: string | null
	decodedData: DecodedProposalData
	onSign: () => void
	onBroadcast: () => void
	onPasteSignatures?: (sigs: PastedSignature[], broadcastState: ImportBroadcastState) => void
	onManualExecute?: () => void
}

function CopyButton({ text, label }: { text: string; label?: string }) {
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
			className="inline-flex shrink-0 items-center gap-1 rounded-md border border-[#e5e7eb] bg-white px-2.5 py-1.5 text-label font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
		>
			<CopyClipboardIcon width={12} height={12} />
			{copied ? 'Copied!' : (label ?? 'Copy')}
		</button>
	)
}

function SectionLabel({ children }: { children: string }) {
	return <p className="mb-2 text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]">{children}</p>
}

type DisplayStatus = ProposalStatus | 'awaiting_enactment'

const STATUS_CONFIG: Record<DisplayStatus, { bg: string; text: string; border: string; dot: string; label: string }> = {
	pending: { bg: '#fffbeb', text: '#d97706', border: '#fde68a', dot: '#d97706', label: 'Pending' },
	approved: { bg: '#eff6ff', text: '#2563eb', border: '#bfdbfe', dot: '#2563eb', label: 'Approved' },
	awaiting_enactment: {
		bg: '#f0fdf9',
		text: '#0f766e',
		border: '#99f6e4',
		dot: '#0f9d7a',
		label: 'Awaiting enactment',
	},
	enacted: { bg: '#ecfdf5', text: '#059669', border: '#a7f3d0', dot: '#059669', label: 'Enacted' },
	canceled: { bg: '#fef2f2', text: '#dc2626', border: '#fecaca', dot: '#dc2626', label: 'Canceled' },
	expired: { bg: '#f9fafb', text: '#6b7280', border: '#e5e7eb', dot: '#6b7280', label: 'Expired' },
}

function StatusBadge({ status }: { status: DisplayStatus }) {
	const s = STATUS_CONFIG[status]
	return (
		<span
			className="inline-flex shrink-0 items-center gap-1.5 rounded-md border px-2.5 py-0.75 text-mono-sm font-medium whitespace-nowrap"
			style={{ background: s.bg, color: s.text, borderColor: s.border }}
		>
			<span className="h-1.5 w-1.5 flex-none rounded-full" style={{ background: s.dot }} aria-hidden="true" />
			{s.label}
		</span>
	)
}

function truncatePubkey(pubkey: string): string {
	return `${pubkey.slice(0, 12)}…${pubkey.slice(-8)}`
}

function deriveProposalTitle(proposal: Proposal, decodedData: DecodedProposalData): string {
	const change = decodedData.signerSetChange
	if (change === null) return `Proposal #${proposal.seqNo}`

	const added = change.rows.filter((r) => r.isAdded).length
	const removed = change.rows.filter((r) => r.isRemoved).length
	const totalAfter = change.rows.filter((r) => r.inAfter).length

	const parts: string[] = []
	if (added > 0 && removed === 0) parts.push(`Add ${added} signer${added > 1 ? 's' : ''}`)
	else if (removed > 0 && added === 0) parts.push(`Remove ${removed} signer${removed > 1 ? 's' : ''}`)
	else if (added > 0 && removed > 0) parts.push('Rotate signers')

	if (change.thresholdBefore !== null && change.thresholdBefore !== change.thresholdAfter) {
		parts.push(`raise threshold to ${change.thresholdAfter} of ${totalAfter}`)
	}

	return parts.length > 0 ? parts.join(' & ') : `Proposal #${proposal.seqNo}`
}

export function ProposalDetail({
	proposal,
	signerPubkey,
	decodedData,
	onSign,
	onBroadcast,
	onPasteSignatures,
	onManualExecute: _onManualExecute,
}: Props) {
	const collectedSignatures = proposal.signatures.length
	const requiredSignatures = proposal.requiredSignatures
	const signaturesProgress =
		requiredSignatures === 0 ? 100 : Math.min((collectedSignatures / requiredSignatures) * 100, 100)

	const { isTerminal, hasQuorum, alreadySigned, canSign } = deriveProposalActions(proposal, signerPubkey)

	const displayStatus: DisplayStatus =
		proposal.status === 'approved' && proposal.broadcastStatus === 'reveal_confirmed'
			? 'awaiting_enactment'
			: proposal.status

	const [bundleCopied, setBundleCopied] = useState(false)
	const [bundleDownloaded, setBundleDownloaded] = useState(false)
	const [showImportModal, setShowImportModal] = useState(false)
	const title = deriveProposalTitle(proposal, decodedData)

	function handleCopyBundle() {
		void writeClipboard(JSON.stringify(proposal, null, 2)).then(() => {
			setBundleCopied(true)
			setTimeout(() => setBundleCopied(false), 2000)
		})
	}

	function handleDownloadBundle() {
		void saveJsonFile(JSON.stringify(proposal, null, 2), `proposal-${proposal.seqNo}.json`).then(() => {
			setBundleDownloaded(true)
			setTimeout(() => setBundleDownloaded(false), 2000)
		})
	}

	return (
		<div className="space-y-4">
			{/* ── Header card ── */}
			<div className="overflow-hidden rounded-xl border border-[#e5e7eb] bg-white shadow-sm">
				<div className="p-6 pb-5">
					<div className="flex items-start justify-between gap-3">
						<div className="min-w-0 flex-1">
							<h2 className="m-0 font-display text-display-md leading-[1.2] text-[#0a0a0a]">{title}</h2>
							<p className="m-0 mt-1 text-body-sm text-[#6b7280]">
								#{proposal.seqNo} · {inferProposalTypeLabel(proposal)} · {proposal.authority}
							</p>
						</div>
						<StatusBadge status={displayStatus} />
					</div>

					{/* Signatures progress */}
					<div className="mt-5">
						<div className="mb-1.5 flex items-center justify-between gap-3">
							<p className="m-0 text-body-sm font-medium text-[#121212]">Signatures</p>
							<p className="m-0 text-body-sm font-medium text-[#121212]">
								{collectedSignatures} / {requiredSignatures} <span className="font-normal text-[#6b7280]">signed</span>
							</p>
						</div>
						<div className="h-1.5 rounded-full bg-[#ebedf0]">
							<div
								className="h-1.5 rounded-full transition-all"
								style={{
									width: `${signaturesProgress}%`,
									background: hasQuorum || isTerminal ? '#0f9d7a' : '#d97706',
								}}
							/>
						</div>
					</div>

					{hasQuorum && (
						<p className="m-0 mt-3 inline-flex items-center gap-1.5 text-body-sm font-medium text-[#0f9d7a]">
							<CheckCircleEmeraldIcon width={14} height={14} className="block shrink-0" />
							Quorum reached — ready to send
						</p>
					)}
				</div>
			</div>

			{/* ── Signer set change ── */}
			{decodedData.signerSetChange !== null && (
				<div className="overflow-hidden rounded-xl border border-[#e5e7eb] bg-white shadow-sm">
					<div className="border-b border-[#f3f4f6] px-6 py-4">
						<SectionLabel>Signer set change</SectionLabel>
					</div>
					<table className="w-full table-fixed border-collapse">
						<thead>
							<tr className="border-b border-[#f3f4f6] bg-[#f9fafb]">
								<th className="w-1/2 px-4 py-2.5 text-left text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]">
									Before
								</th>
								<th className="w-1/2 border-l border-[#f3f4f6] px-4 py-2.5 text-left text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]">
									After
								</th>
							</tr>
						</thead>
						<tbody>
							{decodedData.signerSetChange.rows.map((row, i) => (
								<tr key={i} className="border-b border-[#f3f4f6] last:border-0">
									<td className="px-4 py-2.5 align-top">
										{row.inBefore ? (
											<span
												className={`break-all font-mono text-mono-sm leading-relaxed ${row.isRemoved ? 'text-[#dc2626] line-through' : 'text-[#374151]'}`}
											>
												{row.isRemoved && <span className="mr-1 text-[#dc2626]">−</span>}
												{row.pubkey}
											</span>
										) : (
											<span className="text-[#9ca3af]">—</span>
										)}
									</td>
									<td className="border-l border-[#f3f4f6] px-4 py-2.5 align-top">
										{row.inAfter ? (
											<span
												className={`break-all font-mono text-mono-sm leading-relaxed ${row.isAdded ? 'font-medium text-[#0f9d7a]' : 'text-[#374151]'}`}
											>
												{row.isAdded && <span className="mr-1 text-[#0f9d7a]">+</span>}
												{row.pubkey}
											</span>
										) : (
											<span className="text-[#9ca3af]">—</span>
										)}
									</td>
								</tr>
							))}
							{/* Threshold row */}
							<tr className="border-t border-[#e5e7eb] bg-[#f9fafb]">
								<td className="px-4 py-2.5 text-label">
									<span className="text-[#9ca3af]">Threshold </span>
									{decodedData.signerSetChange.thresholdBefore !== null ? (
										<span className="font-mono font-medium text-[#374151]">
											{decodedData.signerSetChange.thresholdBefore} of{' '}
											{decodedData.signerSetChange.rows.filter((r) => r.inBefore).length}
										</span>
									) : (
										<span className="text-[#9ca3af]">—</span>
									)}
								</td>
								<td className="border-l border-[#f3f4f6] px-4 py-2.5 text-label">
									<span className="text-[#9ca3af]">Threshold </span>
									<span className="font-mono font-medium text-[#374151]">
										{decodedData.signerSetChange.thresholdAfter} of{' '}
										{decodedData.signerSetChange.rows.filter((r) => r.inAfter).length}
									</span>
								</td>
							</tr>
						</tbody>
					</table>
				</div>
			)}

			{/* ── SPS-65 sighash ── */}
			{decodedData.sighashHex !== null && (
				<div className="overflow-hidden rounded-xl border border-[#e5e7eb] bg-white shadow-sm p-6 py-5">
					<SectionLabel>SPS-65 Sighash</SectionLabel>
					<div className="flex items-center gap-2 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
						<span className="min-w-0 flex-1 truncate font-mono text-label text-[#111827]">
							{decodedData.sighashHex}
						</span>
						<CopyButton text={decodedData.sighashHex} />
					</div>
				</div>
			)}

			{/* ── Approvals ── */}
			<div className="overflow-hidden rounded-xl border border-[#e5e7eb] bg-white shadow-sm">
				<div className="flex items-center justify-between border-b border-[#f3f4f6] px-6 py-4">
					<p className="m-0 text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]">
						Approvals · {collectedSignatures} of {requiredSignatures}
					</p>
				</div>
				<div className="divide-y divide-[#f3f4f6]">
					{/* Signed rows — from proposal.signatures */}
					{proposal.signatures.map((sig, i) => {
						const isMe = signerPubkey !== null && sig.signerPubkey.toLowerCase() === signerPubkey.toLowerCase()
						return (
							<div key={i} className="flex items-center gap-3 bg-[#f0fdf9] px-6 py-3">
								<span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-[#0f9d7a]">
									<SignaturePenMutedIcon width={10} height={10} style={{ filter: 'brightness(10)' }} />
								</span>
								<span className="flex-1 font-mono text-label text-[#111827]">{truncatePubkey(sig.signerPubkey)}</span>
								{isMe && (
									<span className="rounded-full border border-accent-border bg-bg-surface px-2 py-0.5 text-[10px] font-medium text-accent-hover">
										YOU
									</span>
								)}
								<button
									type="button"
									title="Copy signature"
									className="shrink-0 rounded-md p-1 text-[#9ca3af] transition hover:text-[#6b7280]"
									onClick={() => void navigator.clipboard.writeText(sig.signatureHex)}
								>
									<CopyClipboardIcon width={13} height={13} />
								</button>
								<span className="shrink-0 text-label text-[#6b7280]">Signed</span>
							</div>
						)
					})}

					{/* Pending rows — signers not yet in signatures */}
					{decodedData.allSigners
						.filter((signer) => !proposal.signatures.some((s) => s.signerPubkey.toLowerCase() === signer.toLowerCase()))
						.map((signer, i) => (
							<div key={`pending-${i}`} className="flex items-center gap-3 px-6 py-3">
								<span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full border-2 border-[#d1d5db] bg-white" />
								<span className="flex-1 font-mono text-label text-[#6b7280]">{truncatePubkey(signer)}</span>
								<span className="shrink-0 text-label text-[#9ca3af]">Pending</span>
							</div>
						))}

					{/* Fallback if no signer data available (allSigners empty, only show signed) */}
					{decodedData.allSigners.length === 0 && proposal.signatures.length === 0 && (
						<div className="px-6 py-4 text-body-sm text-[#9ca3af]">No signatures yet.</div>
					)}
				</div>
			</div>

			{/* ── Broadcast TXIDs ── */}
			{(proposal.commitTxid ?? proposal.revealTxid) && (
				<div className="space-y-4 overflow-hidden rounded-xl border border-[#e5e7eb] bg-white p-6 py-5 shadow-sm">
					{proposal.commitTxid && (
						<div>
							<SectionLabel>Commit TXID</SectionLabel>
							<div className="flex items-center gap-2 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
								<span
									className="min-w-0 flex-1 truncate font-mono text-label text-[#111827]"
									title={proposal.commitTxid}
								>
									{proposal.commitTxid}
								</span>
							</div>
						</div>
					)}
					{proposal.revealTxid && (
						<div>
							<SectionLabel>Reveal TXID</SectionLabel>
							<div className="flex items-center gap-2 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
								<span
									className="min-w-0 flex-1 truncate font-mono text-label text-[#111827]"
									title={proposal.revealTxid}
								>
									{proposal.revealTxid}
								</span>
							</div>
						</div>
					)}
				</div>
			)}

			{/* ── Action buttons ── */}
			<div className="flex items-center justify-between gap-3">
				<div className="flex-1">
					{hasQuorum && (
						<button
							type="button"
							data-testid="e2e-detail-broadcast-button"
							className="inline-flex w-full items-center justify-center gap-1.5 rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-body font-medium text-white transition hover:bg-black"
							onClick={onBroadcast}
						>
							<SendIcon width={14} height={14} />
							Send
						</button>
					)}

					{canSign && (
						<button
							type="button"
							data-testid="e2e-detail-sign-button"
							className="w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-body font-medium text-white transition hover:bg-black"
							onClick={onSign}
						>
							Sign
						</button>
					)}

					{!isTerminal && !hasQuorum && alreadySigned && (
						<div className="rounded-xl border border-[#d1fae5] bg-[#f0fdf4] px-4 py-3">
							<p className="m-0 text-body-sm font-medium text-[#065f46]">
								You have signed this proposal. Waiting for other signers to reach quorum.
							</p>
						</div>
					)}
				</div>

				{/* Utility buttons */}
				<div className="flex shrink-0 items-center gap-1.5">
					{!isTerminal && onPasteSignatures && (
						<div className="group relative">
							<button
								type="button"
								onClick={() => setShowImportModal(true)}
								className="inline-flex items-center gap-1 rounded-md border border-[#e5e7eb] bg-white px-2.5 py-1.5 text-label font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
							>
								<ImportJsonIcon width={12} height={12} className="text-current" />
							</button>
							<span className="pointer-events-none absolute bottom-full left-1/2 mb-1.5 -translate-x-1/2 whitespace-nowrap rounded-md bg-[#111827] px-2 py-1 text-mono-sm text-white opacity-0 transition-opacity group-hover:opacity-100">
								Import signatures
							</span>
						</div>
					)}
					<div className="group relative">
						<button
							type="button"
							onClick={handleCopyBundle}
							className="inline-flex items-center gap-1 rounded-md border border-[#e5e7eb] bg-white px-2.5 py-1.5 text-label font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
						>
							{bundleCopied ? (
								<span className="text-[#0f9d7a]">Copied!</span>
							) : (
								<CopyClipboardIcon width={12} height={12} className="text-current" />
							)}
						</button>
						{!bundleCopied && (
							<span className="pointer-events-none absolute bottom-full left-1/2 mb-1.5 -translate-x-1/2 whitespace-nowrap rounded-md bg-[#111827] px-2 py-1 text-mono-sm text-white opacity-0 transition-opacity group-hover:opacity-100">
								Copy bundle
							</span>
						)}
					</div>
					<div className="group relative">
						<button
							type="button"
							onClick={handleDownloadBundle}
							className="inline-flex items-center gap-1 rounded-md border border-[#e5e7eb] bg-white px-2.5 py-1.5 text-label font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
						>
							{bundleDownloaded ? (
								<span className="text-[#0f9d7a]">Saved!</span>
							) : (
								<DownloadIcon width={12} height={12} className="text-current" />
							)}
						</button>
						{!bundleDownloaded && (
							<span className="pointer-events-none absolute bottom-full left-1/2 mb-1.5 -translate-x-1/2 whitespace-nowrap rounded-md bg-[#111827] px-2 py-1 text-mono-sm text-white opacity-0 transition-opacity group-hover:opacity-100">
								Download bundle
							</span>
						)}
					</div>
				</div>
			</div>

			{showImportModal && onPasteSignatures && (
				<ImportBundleModal
					existingSignatures={proposal.signatures}
					existingBroadcastStatus={proposal.broadcastStatus}
					existingCommitTxid={proposal.commitTxid ?? null}
					existingRevealTxid={proposal.revealTxid ?? null}
					onImport={onPasteSignatures}
					onClose={() => setShowImportModal(false)}
				/>
			)}
		</div>
	)
}
