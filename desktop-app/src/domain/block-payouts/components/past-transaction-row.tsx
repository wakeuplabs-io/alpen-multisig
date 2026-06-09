import { useState } from 'react'
import { Paginator } from '@/components/paginator'
import type { PastBlockPayoutTx } from '../model/block-payouts.types'
import { ConflictingInputIcon } from './conflicting-input-icon'

type Props = {
	tx: PastBlockPayoutTx
	onRebroadcast: () => void
	onCopyRawTx: () => void
}

const INPUTS_PER_PAGE = 10

function truncate(str: string, head = 10, tail = 8): string {
	if (str.length <= head + tail + 3) return str
	return `${str.slice(0, head)}…${str.slice(-tail)}`
}

function formatDate(d: Date): string {
	return d.toLocaleString(undefined, {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
		hour: '2-digit',
		minute: '2-digit',
	})
}

function formatSats(sats: number): string {
	return sats.toLocaleString() + ' sats'
}

export function PastTransactionRow({ tx, onRebroadcast, onCopyRawTx }: Props) {
	const isConfirmed = tx.status === 'confirmed'
	const [expanded, setExpanded] = useState(false)
	const [inputsPage, setInputsPage] = useState(1)
	const totalInputPages = Math.ceil(tx.inputs.length / INPUTS_PER_PAGE)
	const pagedInputs = tx.inputs.slice((inputsPage - 1) * INPUTS_PER_PAGE, inputsPage * INPUTS_PER_PAGE)

	return (
		<div className="rounded-xl border border-[#e5e7eb] bg-white">
			<div className="flex items-center gap-4 px-5 py-3.5">
				{/* TX ID */}
				<div className="min-w-0 flex-1">
					<span className="font-mono text-[13px] font-medium text-[#0a0a0a]">{truncate(tx.id, 12, 6)}</span>
				</div>

				{/* Status badge */}
				<span
					className="inline-flex shrink-0 items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-medium"
					style={
						isConfirmed
							? { background: '#ecfdf5', color: '#059669', borderColor: '#a7f3d0' }
							: { background: '#eff6ff', color: '#2563eb', borderColor: '#bfdbfe' }
					}
				>
					<span
						className="h-1.5 w-1.5 flex-none rounded-full"
						style={{ background: isConfirmed ? '#059669' : '#2563eb' }}
						aria-hidden="true"
					/>
					{isConfirmed ? 'Confirmed' : 'Unconfirmed'}
				</span>

				{/* Timestamp */}
				<span className="w-44 shrink-0 text-right text-[12px] text-[#6b7280]">
					{isConfirmed && tx.blockTimestamp ? formatDate(tx.blockTimestamp) : formatDate(tx.broadcastAt)}
				</span>

				{/* Actions */}
				<div className="flex shrink-0 items-center gap-1.5">
					{!isConfirmed && (
						<>
							<button
								type="button"
								className="inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-3 py-1.5 text-[12px] font-medium text-[#374151] transition hover:border-[#d1d5db] hover:bg-[#f9fafb]"
								onClick={onRebroadcast}
							>
								<svg width={12} height={12} viewBox="0 0 24 24" fill="none" aria-hidden>
									<polyline
										points="1 4 1 10 7 10"
										stroke="currentColor"
										strokeWidth="1.5"
										strokeLinecap="round"
										strokeLinejoin="round"
									/>
									<path d="M3.51 15a9 9 0 1 0 .49-4.98" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
								</svg>
								Rebroadcast
							</button>
							<button
								type="button"
								title="Copy raw transaction"
								aria-label="Copy raw transaction"
								className="flex h-7 w-7 items-center justify-center rounded-md border border-[#e5e7eb] bg-white text-[#6b7280] transition hover:border-[#d1d5db] hover:bg-[#f9fafb] hover:text-[#374151]"
								onClick={onCopyRawTx}
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
						</>
					)}

					{/* Toggle inputs */}
					{tx.inputs.length > 0 && (
						<button
							type="button"
							title={expanded ? 'Hide inputs' : 'Show inputs'}
							aria-label={expanded ? 'Hide inputs' : 'Show inputs'}
							className="flex h-7 w-7 items-center justify-center rounded-md border border-[#e5e7eb] bg-white text-[#6b7280] transition hover:border-[#d1d5db] hover:bg-[#f9fafb] hover:text-[#374151]"
							onClick={() => setExpanded((v) => !v)}
						>
							<svg
								width={12}
								height={12}
								viewBox="0 0 24 24"
								fill="none"
								aria-hidden
								className={`transition-transform ${expanded ? 'rotate-180' : ''}`}
							>
								<path
									d="M6 9l6 6 6-6"
									stroke="currentColor"
									strokeWidth="1.5"
									strokeLinecap="round"
									strokeLinejoin="round"
								/>
							</svg>
						</button>
					)}
				</div>
			</div>

			{/* Inputs section */}
			{expanded && tx.inputs.length > 0 && (
				<div className="border-t border-[#f3f4f6] px-5 pb-4 pt-3">
					<p className="m-0 mb-1.5 text-[12px] font-semibold uppercase tracking-wider text-[#9ca3af]">Inputs</p>
					<div className="flex flex-col gap-1">
						{pagedInputs.map((input) => (
							<div key={input.outpoint} className="flex items-center gap-1.5">
								<span className="font-mono text-[12px] text-[#374151]">{truncate(input.outpoint, 16, 6)}</span>
								<span className="text-[11px] text-[#9ca3af]">({formatSats(input.amount)})</span>
								{input.isConflicting && <ConflictingInputIcon />}
							</div>
						))}
					</div>
					<Paginator
						page={inputsPage}
						totalPages={totalInputPages}
						onPageChange={setInputsPage}
						className="mt-3 flex items-center gap-1.5"
					/>
				</div>
			)}
		</div>
	)
}
