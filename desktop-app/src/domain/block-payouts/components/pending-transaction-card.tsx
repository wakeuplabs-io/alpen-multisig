import { writeClipboard } from '@/api/tauri-bridge'
import type { PendingBlockPayoutTx } from '../model/block-payouts.types'
import { ConflictingInputIcon } from './conflicting-input-icon'

type Props = {
	tx: PendingBlockPayoutTx
	onSign: () => void
	onPasteSignatures: () => void
	onExport: () => void
	onCopySignatures: () => void
}

function formatTimeRemaining(expiresAt: Date): { label: string; urgent: boolean } {
	const ms = expiresAt.getTime() - Date.now()
	if (ms <= 0) return { label: 'Expired', urgent: true }
	const totalMinutes = Math.floor(ms / 60000)
	const hours = Math.floor(totalMinutes / 60)
	const minutes = totalMinutes % 60
	const urgent = hours < 24
	if (hours >= 24) {
		const days = Math.floor(hours / 24)
		const remHours = hours % 24
		return { label: `${days}d ${remHours}h`, urgent: false }
	}
	return { label: `${hours}h ${minutes}m`, urgent }
}

function truncate(str: string, head = 10, tail = 8): string {
	if (str.length <= head + tail + 3) return str
	return `${str.slice(0, head)}…${str.slice(-tail)}`
}

function formatSats(sats: number): string {
	return sats.toLocaleString() + ' sats'
}

export function PendingTransactionCard({ tx, onSign, onPasteSignatures, onExport, onCopySignatures }: Props) {
	const { label: timeLabel, urgent } = formatTimeRemaining(tx.expiresAt)
	const progress =
		tx.signaturesRequired === 0 ? 0 : Math.min((tx.signaturesReceived / tx.signaturesRequired) * 100, 100)
	const hasQuorum = tx.signaturesReceived >= tx.signaturesRequired

	function handleCopyId() {
		void writeClipboard(tx.id)
	}

	return (
		<div className="rounded-xl border border-[#e5e7eb] bg-white px-5 py-4 shadow-sm">
			{/* Header row */}
			<div className="flex items-start justify-between gap-3">
				<div className="min-w-0 flex-1">
					<div className="flex items-center gap-2">
						<span className="font-mono text-body-sm font-medium text-[#0a0a0a]">{truncate(tx.id, 12, 6)}</span>
						<button
							type="button"
							className="rounded p-0.5 text-[#9ca3af] transition hover:text-[#6b7280]"
							onClick={handleCopyId}
							title="Copy transaction ID"
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
					</div>
					<p className="m-0 mt-0.5 text-label text-[#6b7280]">
						{tx.inputs.length} input{tx.inputs.length !== 1 ? 's' : ''}
					</p>
				</div>

				{/* Expiry badge */}
				<span
					className="inline-flex shrink-0 items-center gap-1 rounded-md border px-2 py-0.5 text-mono-sm font-medium"
					style={
						urgent
							? { background: '#fef2f2', color: '#dc2626', borderColor: '#fecaca' }
							: { background: '#f9fafb', color: '#6b7280', borderColor: '#e5e7eb' }
					}
				>
					<svg width={11} height={11} viewBox="0 0 24 24" fill="none" aria-hidden>
						<circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="1.5" />
						<path d="M12 7v5l3 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
					</svg>
					{timeLabel}
				</span>
			</div>

			{/* Signature progress */}
			<div className="mt-4">
				<div className="mb-1.5 flex items-center justify-between gap-2">
					<span className="text-body-sm font-medium text-[#0a0a0a]">Signatures</span>
					<span className="text-body-sm font-medium text-[#0a0a0a]">
						{tx.signaturesReceived} / {tx.signaturesRequired}
					</span>
				</div>
				<div className="h-1.5 rounded-full bg-[#ebedf0]">
					<div
						className="h-1.5 rounded-full transition-all"
						style={{ width: `${progress}%`, background: hasQuorum ? '#0f9d7a' : '#d97706' }}
					/>
				</div>
			</div>

			{/* Inputs list */}
			<div className="mt-4">
				<p className="m-0 mb-1.5 text-label font-semibold uppercase tracking-wider text-[#9ca3af]">Inputs</p>
				<div className="flex flex-col gap-1">
					{tx.inputs.map((input) => (
						<div key={input.outpoint} className="flex items-center gap-1.5">
							<span className="font-mono text-label text-[#374151]">{truncate(input.outpoint, 16, 6)}</span>
							<span className="text-mono-sm text-[#9ca3af]">({formatSats(input.amount)})</span>
							{input.isConflicting && <ConflictingInputIcon />}
						</div>
					))}
				</div>
			</div>

			{/* Footer row */}
			<div className="mt-4 flex items-center justify-between gap-3 border-t border-[#f3f4f6] pt-3">
				{/* Signed status or Sign button */}
				{tx.signedByCurrentUser ? (
					<span className="inline-flex items-center gap-1.5 text-body-sm font-medium text-[#059669]">
						<svg width={14} height={14} viewBox="0 0 24 24" fill="none" aria-hidden>
							<path d="M5 12l5 5L20 7" stroke="#059669" strokeWidth="2" strokeLinecap="round" />
						</svg>
						Signed
					</span>
				) : (
					<button
						type="button"
						className="inline-flex items-center rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-3.5 py-1.5 text-body-sm font-medium text-white transition hover:bg-[#2a2a2a] active:scale-[0.98]"
						onClick={onSign}
					>
						Sign
					</button>
				)}

				{/* Secondary actions */}
				<div className="flex items-center gap-1">
					<IconButton label="Paste signatures" onClick={onPasteSignatures}>
						<svg width={14} height={14} viewBox="0 0 24 24" fill="none" aria-hidden>
							<rect x="8" y="2" width="8" height="4" rx="1" stroke="currentColor" strokeWidth="1.5" />
							<path
								d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"
								stroke="currentColor"
								strokeWidth="1.5"
							/>
							<path d="M9 12h6M9 16h4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
						</svg>
					</IconButton>
					<IconButton label="Copy signatures" onClick={onCopySignatures}>
						<svg width={14} height={14} viewBox="0 0 24 24" fill="none" aria-hidden>
							<rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" strokeWidth="1.5" />
							<path
								d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
								stroke="currentColor"
								strokeWidth="1.5"
							/>
						</svg>
					</IconButton>
					<IconButton label="Export raw transaction" onClick={onExport}>
						<svg width={14} height={14} viewBox="0 0 24 24" fill="none" aria-hidden>
							<path
								d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
								stroke="currentColor"
								strokeWidth="1.5"
								strokeLinecap="round"
							/>
							<polyline
								points="7 10 12 15 17 10"
								stroke="currentColor"
								strokeWidth="1.5"
								strokeLinecap="round"
								strokeLinejoin="round"
							/>
							<line x1="12" y1="15" x2="12" y2="3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
						</svg>
					</IconButton>
				</div>
			</div>
		</div>
	)
}

function IconButton({ label, onClick, children }: { label: string; onClick: () => void; children: React.ReactNode }) {
	return (
		<button
			type="button"
			title={label}
			aria-label={label}
			className="flex h-7 w-7 items-center justify-center rounded-md border border-[#e5e7eb] bg-white text-[#6b7280] transition hover:border-[#d1d5db] hover:bg-[#f9fafb] hover:text-[#374151]"
			onClick={onClick}
		>
			{children}
		</button>
	)
}
