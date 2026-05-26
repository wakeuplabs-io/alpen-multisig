import { useState } from 'react'
import type { PasteResult } from '../hooks/use-block-payouts'

type Props = {
	txId: string
	onSubmit: (txId: string, raw: string) => PasteResult
	onClose: () => void
}

export function PasteSignaturesModal({ txId, onSubmit, onClose }: Props) {
	const [raw, setRaw] = useState('')
	const [result, setResult] = useState<PasteResult | null>(null)

	function handleSubmit() {
		const res = onSubmit(txId, raw)
		setResult(res)
		if (res.ok) {
			onClose()
		}
	}

	function buildErrorMessage(invalid: string[]): string {
		if (invalid.length === 1) {
			return `Warning: Invalid signature. Please provide a valid signature.\n${invalid[0]}`
		}
		return `Warning: Invalid signatures. Please provide valid signatures.\n${invalid.join('\n')}`
	}

	const errorMessage = result && !result.ok ? buildErrorMessage(result.invalidSignatures) : null

	function handleCopyError() {
		if (errorMessage) void navigator.clipboard.writeText(errorMessage)
	}

	return (
		<Backdrop onClose={onClose}>
			<div className="flex flex-col gap-5">
				<div>
					<h2 className="m-0 font-['BIZ_UDPMincho'] text-[22px] font-normal text-[#0a0a0a]">Paste signatures</h2>
					<p className="m-0 mt-1 text-[13px] text-[#6b7280]">
						Paste one signature per line. Valid signatures will be added to the transaction.
					</p>
				</div>

				<textarea
					className="h-36 w-full resize-none rounded-xl border border-[#e5e7eb] bg-[#f8f8fb] px-4 py-3 font-mono text-[12px] text-[#0a0a0a] outline-none transition focus:border-[#0a0a0a] focus:bg-white"
					placeholder="Paste signatures here, one per line…"
					value={raw}
					onChange={(e) => {
						setRaw(e.target.value)
						setResult(null)
					}}
					spellCheck={false}
				/>

				{errorMessage && (
					<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
						<pre className="m-0 whitespace-pre-wrap font-mono text-[12px] text-[#991b1b]">{errorMessage}</pre>
						<button
							type="button"
							className="mt-2.5 inline-flex items-center gap-1.5 rounded-md border border-[#fca5a5] bg-white px-3 py-1 text-[12px] font-medium text-[#b91c1c] transition hover:bg-[#fef2f2]"
							onClick={handleCopyError}
						>
							<svg width={12} height={12} viewBox="0 0 24 24" fill="none" aria-hidden>
								<rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" strokeWidth="1.5" />
								<path
									d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"
									stroke="currentColor"
									strokeWidth="1.5"
								/>
							</svg>
							Copy error
						</button>
					</div>
				)}

				<div className="flex items-center justify-end gap-2.5">
					<button
						type="button"
						className="rounded-lg border border-[#e5e7eb] bg-white px-4 py-2 text-[13px] font-medium text-[#374151] transition hover:bg-[#f9fafb]"
						onClick={onClose}
					>
						Cancel
					</button>
					<button
						type="button"
						disabled={raw.trim().length === 0}
						className="inline-flex items-center rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-[13px] font-medium text-white transition hover:bg-[#2a2a2a] active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50"
						onClick={handleSubmit}
					>
						Import
					</button>
				</div>
			</div>
		</Backdrop>
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
