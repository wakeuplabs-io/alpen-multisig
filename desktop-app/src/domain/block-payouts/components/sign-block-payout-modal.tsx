import type { PendingBlockPayoutTx } from '../model/block-payouts.types'

type Props = {
	tx: PendingBlockPayoutTx
	onSign: () => void
	onClose: () => void
}

function truncate(str: string, head = 12, tail = 6): string {
	if (str.length <= head + tail + 3) return str
	return `${str.slice(0, head)}…${str.slice(-tail)}`
}

export function SignBlockPayoutModal({ tx, onSign, onClose }: Props) {
	const remaining = tx.signaturesRequired - tx.signaturesReceived

	return (
		<Backdrop onClose={onClose}>
			<div className="flex flex-col gap-5">
				<div>
					<h2 className="m-0 font-display text-display-sm font-normal text-[#0a0a0a]">Sign transaction</h2>
					<p className="m-0 mt-1 text-body-sm text-[#6b7280]">Review the transaction details before signing.</p>
				</div>

				{/* Summary */}
				<div className="rounded-xl border border-[#e5e7eb] bg-bg-base px-4 py-3.5">
					<dl className="flex flex-col gap-2">
						<Row label="Transaction ID" value={<span className="font-mono text-body-sm">{truncate(tx.id)}</span>} />
						<Row label="Inputs" value={`${tx.inputs.length} input${tx.inputs.length !== 1 ? 's' : ''}`} />
						<Row
							label="Signatures"
							value={
								<span>
									{tx.signaturesReceived} / {tx.signaturesRequired}{' '}
									<span className="text-[#6b7280]">({remaining} more needed)</span>
								</span>
							}
						/>
					</dl>
				</div>

				{/* Warning */}
				<div className="flex items-start gap-2.5 rounded-xl border border-accent-border bg-highlight-surface px-4 py-3">
					<svg
						width={15}
						height={15}
						viewBox="0 0 24 24"
						fill="none"
						aria-hidden
						className="mt-0.5 shrink-0 text-emphasis-soft"
					>
						<path
							d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"
							stroke="currentColor"
							strokeWidth="1.5"
							strokeLinejoin="round"
						/>
						<path d="M12 9v4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
						<circle cx="12" cy="17" r="1" fill="currentColor" />
					</svg>
					<p className="m-0 text-body-sm text-emphasis">
						You are about to sign this <strong>block_payouts</strong> transaction. This action cannot be undone.
					</p>
				</div>

				{/* Actions */}
				<div className="flex items-center justify-end gap-2.5">
					<button
						type="button"
						className="rounded-lg border border-[#e5e7eb] bg-white px-4 py-2 text-body-sm font-medium text-[#374151] transition hover:bg-[#f9fafb]"
						onClick={onClose}
					>
						Cancel
					</button>
					<button
						type="button"
						className="inline-flex items-center gap-1.5 rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-body-sm font-medium text-white transition hover:bg-[#2a2a2a] active:scale-[0.98]"
						onClick={onSign}
					>
						<svg width={13} height={13} viewBox="0 0 24 24" fill="none" aria-hidden>
							<path d="M12 20h9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
							<path
								d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"
								stroke="currentColor"
								strokeWidth="1.5"
								strokeLinejoin="round"
							/>
						</svg>
						Sign
					</button>
				</div>
			</div>
		</Backdrop>
	)
}

function Row({ label, value }: { label: string; value: React.ReactNode }) {
	return (
		<div className="flex items-center justify-between gap-3">
			<dt className="text-label text-[#6b7280]">{label}</dt>
			<dd className="m-0 text-body-sm font-medium text-[#0a0a0a]">{value}</dd>
		</div>
	)
}

function Backdrop({ children, onClose }: { children: React.ReactNode; onClose: () => void }) {
	return (
		<div
			className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 px-4"
			onClick={(e) => {
				if (e.target === e.currentTarget) onClose()
			}}
		>
			<div className="w-full max-w-120 rounded-2xl border border-[#e5e7eb] bg-white p-6 shadow-xl">{children}</div>
		</div>
	)
}
