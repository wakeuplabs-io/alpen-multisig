import { useState, useRef } from 'react'
import type { BlockPayoutInput, FalseClaimReport, ParsedReport } from '../model/block-payouts.types'

const MAX_INPUTS = 50

const ESTIMATED_VBYTES_OVERHEAD = 11
const ESTIMATED_VBYTES_PER_INPUT = 68
const ESTIMATED_VBYTES_OUTPUT = 31

type Props = {
	walletBalanceSats: number
	onConfirm: (inputs: BlockPayoutInput[], feeRateSatPerVb: number) => void
	onClose: () => void
}

type Step = 1 | 2 | 3 | 4

// Outpoints ending in :0 are treated as spent in the mock
function isSpent(outpoint: string): boolean {
	return outpoint.endsWith(':0')
}

function parseReports(raw: string): ParsedReport[] {
	return raw
		.split('\n')
		.map((line) => line.trim())
		.filter(Boolean)
		.map((line) => {
			try {
				const obj = JSON.parse(line) as Record<string, unknown>
				if (!obj.claimId || !obj.outpoint || !obj.amount || !obj.proof) {
					return {
						valid: false,
						reason: 'Missing required fields (claimId, outpoint, amount, proof)',
						raw: line,
					} as ParsedReport
				}
				if (typeof obj.proof !== 'string' || obj.proof.trim() === '') {
					return { valid: false, reason: 'Invalid or empty proof', raw: line } as ParsedReport
				}
				return {
					valid: true,
					report: {
						claimId: String(obj.claimId),
						outpoint: String(obj.outpoint),
						amount: Number(obj.amount),
						proof: String(obj.proof),
					},
				} as ParsedReport
			} catch {
				return { valid: false, reason: 'Invalid JSON', raw: line } as ParsedReport
			}
		})
}

function reportsToInputs(reports: FalseClaimReport[]): BlockPayoutInput[] {
	return reports
		.filter((r) => !isSpent(r.outpoint))
		.map((r) => ({
			outpoint: r.outpoint,
			amount: r.amount,
			claimId: r.claimId,
			isConflicting: false,
		}))
}

function formatSats(sats: number): string {
	return sats.toLocaleString() + ' sats'
}

function truncate(str: string, head = 16, tail = 6): string {
	if (str.length <= head + tail + 3) return str
	return `${str.slice(0, head)}…${str.slice(-tail)}`
}

export function CreateBlockPayoutModal({ walletBalanceSats, onConfirm, onClose }: Props) {
	const [step, setStep] = useState<Step>(1)
	const [rawReports, setRawReports] = useState('')
	const [parsed, setParsed] = useState<ParsedReport[]>([])
	const [inputs, setInputs] = useState<BlockPayoutInput[]>([])
	const [feeRate, setFeeRate] = useState(1.0)
	const fileRef = useRef<HTMLInputElement>(null)

	const invalidReports = parsed.filter((p) => !p.valid)
	const exceedsLimit = inputs.length > MAX_INPUTS
	const estimatedVsize =
		ESTIMATED_VBYTES_OVERHEAD + (inputs.length + 1) * ESTIMATED_VBYTES_PER_INPUT + ESTIMATED_VBYTES_OUTPUT
	const estimatedFeeSats = Math.ceil(estimatedVsize * feeRate)
	const insufficientBalance = estimatedFeeSats > walletBalanceSats

	function handleParseReports() {
		const results = parseReports(rawReports)
		setParsed(results)
		const valid = results.filter((p): p is Extract<ParsedReport, { valid: true }> => p.valid).map((p) => p.report)
		setInputs(reportsToInputs(valid))
		setStep(2)
	}

	function handleFileLoad(e: React.ChangeEvent<HTMLInputElement>) {
		const file = e.target.files?.[0]
		if (!file) return
		const reader = new FileReader()
		reader.onload = (ev) => {
			setRawReports(String(ev.target?.result ?? ''))
		}
		reader.readAsText(file)
	}

	function handleRemoveInput(outpoint: string) {
		setInputs((prev) => prev.filter((i) => i.outpoint !== outpoint))
	}

	function handleConfirm() {
		onConfirm(inputs, feeRate)
	}

	return (
		<Backdrop onClose={onClose}>
			<div className="flex flex-col gap-5">
				{/* Header */}
				<div>
					<div className="mb-3 flex items-center gap-1.5">
						{([1, 2, 3, 4] as Step[]).map((s) => (
							<div
								key={s}
								className="h-1 flex-1 rounded-full transition-all"
								style={{ background: s <= step ? '#0a0a0a' : '#e5e7eb' }}
							/>
						))}
					</div>
					<h2 className="m-0 font-display text-display-sm font-normal text-[#0a0a0a]">
						{step === 1 && 'Load false claim reports'}
						{step === 2 && 'Review inputs'}
						{step === 3 && 'Set fee rate'}
						{step === 4 && 'Confirm transaction'}
					</h2>
					<p className="m-0 mt-1 text-label text-[#6b7280]">Step {step} of 4</p>
				</div>

				{/* Step 1 */}
				{step === 1 && (
					<>
						<div>
							<p className="m-0 mb-2 text-body-sm text-[#374151]">
								Paste false claim reports below (one JSON object per line), or upload a file.
							</p>
							<textarea
								className="h-40 w-full resize-none rounded-xl border border-[#e5e7eb] bg-bg-base px-4 py-3 font-mono text-label text-[#0a0a0a] outline-none transition focus:border-[#0a0a0a] focus:bg-white"
								placeholder={'{"claimId":"claim-001","outpoint":"abc...def:1","amount":100000,"proof":"..."}'}
								value={rawReports}
								onChange={(e) => setRawReports(e.target.value)}
								spellCheck={false}
							/>
							<div className="mt-2 flex items-center gap-2">
								<button
									type="button"
									className="text-label text-[#6b7280] underline underline-offset-2 transition hover:text-[#374151]"
									onClick={() => fileRef.current?.click()}
								>
									Upload file instead
								</button>
								<input ref={fileRef} type="file" accept=".json,.txt" className="hidden" onChange={handleFileLoad} />
							</div>
						</div>
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
								disabled={rawReports.trim().length === 0}
								className="rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-body-sm font-medium text-white transition hover:bg-[#2a2a2a] disabled:cursor-not-allowed disabled:opacity-50"
								onClick={handleParseReports}
							>
								Next
							</button>
						</div>
					</>
				)}

				{/* Step 2 */}
				{step === 2 && (
					<>
						{invalidReports.length > 0 && (
							<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
								<p className="m-0 text-body-sm font-medium text-[#991b1b]">
									{invalidReports.length} report{invalidReports.length !== 1 ? 's' : ''} ignored (invalid proof or
									missing fields)
								</p>
							</div>
						)}

						{exceedsLimit && (
							<div className="rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
								<p className="m-0 text-body-sm font-medium text-[#991b1b]">
									Your transaction exceeds the size limit, please remove one or more inputs to reduce its size.
								</p>
							</div>
						)}

						<div>
							<p className="m-0 mb-2 text-body-sm font-medium text-[#374151]">
								{inputs.length} input{inputs.length !== 1 ? 's' : ''} — remove any to adjust the transaction
							</p>
							{inputs.length === 0 ? (
								<div className="rounded-xl border border-[#e5e7eb] bg-bg-base px-4 py-4 text-center text-body-sm text-[#9ca3af]">
									No valid unspent inputs found.
								</div>
							) : (
								<div className="flex max-h-52 flex-col gap-1 overflow-y-auto rounded-xl border border-[#e5e7eb] bg-bg-base p-3">
									{inputs.map((input) => (
										<div
											key={input.outpoint}
											className="flex items-center justify-between gap-3 rounded-lg bg-white px-3 py-2 shadow-sm"
										>
											<div className="min-w-0">
												<span className="font-mono text-label text-[#374151]">{truncate(input.outpoint)}</span>
												<span className="ml-2 text-mono-sm text-[#9ca3af]">({formatSats(input.amount)})</span>
											</div>
											<button
												type="button"
												aria-label="Remove input"
												className="shrink-0 rounded-md p-1 text-[#9ca3af] transition hover:bg-[#fef2f2] hover:text-[#dc2626]"
												onClick={() => handleRemoveInput(input.outpoint)}
											>
												<svg width={13} height={13} viewBox="0 0 24 24" fill="none" aria-hidden>
													<line
														x1="18"
														y1="6"
														x2="6"
														y2="18"
														stroke="currentColor"
														strokeWidth="1.5"
														strokeLinecap="round"
													/>
													<line
														x1="6"
														y1="6"
														x2="18"
														y2="18"
														stroke="currentColor"
														strokeWidth="1.5"
														strokeLinecap="round"
													/>
												</svg>
											</button>
										</div>
									))}
								</div>
							)}
						</div>

						<div className="flex items-center justify-between gap-2.5">
							<button
								type="button"
								className="rounded-lg border border-[#e5e7eb] bg-white px-4 py-2 text-body-sm font-medium text-[#374151] transition hover:bg-[#f9fafb]"
								onClick={() => setStep(1)}
							>
								Back
							</button>
							<button
								type="button"
								disabled={inputs.length === 0 || exceedsLimit}
								className="rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-body-sm font-medium text-white transition hover:bg-[#2a2a2a] disabled:cursor-not-allowed disabled:opacity-50"
								onClick={() => setStep(3)}
							>
								Next
							</button>
						</div>
					</>
				)}

				{/* Step 3 */}
				{step === 3 && (
					<>
						<div>
							<label className="mb-1.5 block text-body-sm font-medium text-[#374151]" htmlFor="fee-rate">
								Fee rate (sat/vB)
							</label>
							<input
								id="fee-rate"
								type="number"
								min={0.1}
								max={10000}
								step={0.1}
								value={feeRate}
								onChange={(e) => setFeeRate(Math.max(0.1, Math.min(10000, Number(e.target.value))))}
								className="w-full rounded-xl border border-[#e5e7eb] bg-bg-base px-4 py-2.5 text-body-sm text-[#0a0a0a] outline-none transition focus:border-[#0a0a0a] focus:bg-white"
							/>
							<p className="m-0 mt-2 text-label text-[#6b7280]">Range: 0.1 – 10,000 sat/vB in increments of 0.1</p>
						</div>

						<div className="flex items-start gap-2.5 rounded-xl border border-[#e5e7eb] bg-bg-base px-4 py-3">
							<svg
								width={14}
								height={14}
								viewBox="0 0 24 24"
								fill="none"
								aria-hidden
								className="mt-0.5 shrink-0 text-[#6b7280]"
							>
								<circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="1.5" />
								<path d="M12 8v4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
								<circle cx="12" cy="16" r="1" fill="currentColor" />
							</svg>
							<p className="m-0 text-body-sm text-[#6b7280]">
								Fee paid from <strong className="text-[#374151]">Admin Wallet</strong>.
							</p>
						</div>

						<div className="rounded-xl border border-[#e5e7eb] bg-bg-base px-4 py-3">
							<div className="flex items-center justify-between gap-3">
								<span className="text-label text-[#6b7280]">Estimated fee</span>
								<span className="text-body-sm font-medium text-[#0a0a0a]">{formatSats(estimatedFeeSats)}</span>
							</div>
							<div className="mt-1.5 flex items-center justify-between gap-3">
								<span className="text-label text-[#6b7280]">Admin Wallet balance</span>
								<span className="text-body-sm font-medium text-[#0a0a0a]">{formatSats(walletBalanceSats)}</span>
							</div>
						</div>

						{insufficientBalance && (
							<div className="flex items-start gap-2.5 rounded-xl border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
								<svg
									width={14}
									height={14}
									viewBox="0 0 24 24"
									fill="none"
									aria-hidden
									className="mt-0.5 shrink-0 text-[#dc2626]"
								>
									<circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="1.5" />
									<path d="M12 8v4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
									<circle cx="12" cy="16" r="1" fill="currentColor" />
								</svg>
								<p className="m-0 text-body-sm text-[#991b1b]">
									Admin Wallet balance is insufficient to cover the estimated fee. Lower the fee rate or fund the
									wallet.
								</p>
							</div>
						)}

						<div className="flex items-center justify-between gap-2.5">
							<button
								type="button"
								className="rounded-lg border border-[#e5e7eb] bg-white px-4 py-2 text-body-sm font-medium text-[#374151] transition hover:bg-[#f9fafb]"
								onClick={() => setStep(2)}
							>
								Back
							</button>
							<button
								type="button"
								disabled={insufficientBalance}
								className="rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-body-sm font-medium text-white transition hover:bg-[#2a2a2a] disabled:cursor-not-allowed disabled:opacity-50"
								onClick={() => setStep(4)}
							>
								Next
							</button>
						</div>
					</>
				)}

				{/* Step 4 */}
				{step === 4 && (
					<>
						<div className="rounded-xl border border-[#e5e7eb] bg-bg-base px-4 py-4">
							<dl className="flex flex-col gap-2.5">
								<SummaryRow label="block_payouts inputs" value={`${inputs.length}`} />
								<SummaryRow label="Fee rate" value={`${feeRate} sat/vB`} />
								<SummaryRow label="Fee source" value="Admin Wallet" />
								<SummaryRow label="Change destination" value="First unused change address" />
							</dl>
						</div>

						<div className="flex items-start gap-2.5 rounded-xl border border-[#e5e7eb] bg-bg-base px-4 py-3">
							<svg
								width={14}
								height={14}
								viewBox="0 0 24 24"
								fill="none"
								aria-hidden
								className="mt-0.5 shrink-0 text-[#6b7280]"
							>
								<circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="1.5" />
								<path d="M12 8v4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
								<circle cx="12" cy="16" r="1" fill="currentColor" />
							</svg>
							<p className="m-0 text-body-sm text-[#92400e]">
								Clicking <strong>Confirm</strong> will create this transaction and add your signature to it.
							</p>
						</div>

						<div className="flex items-center justify-between gap-2.5">
							<button
								type="button"
								className="rounded-lg border border-[#e5e7eb] bg-white px-4 py-2 text-body-sm font-medium text-[#374151] transition hover:bg-[#f9fafb]"
								onClick={() => setStep(3)}
							>
								Back
							</button>
							<button
								type="button"
								className="inline-flex items-center gap-1.5 rounded-lg border border-[#0a0a0a] bg-[#0a0a0a] px-4 py-2 text-body-sm font-medium text-white transition hover:bg-[#2a2a2a] active:scale-[0.98]"
								onClick={handleConfirm}
							>
								Confirm
							</button>
						</div>
					</>
				)}
			</div>
		</Backdrop>
	)
}

function SummaryRow({ label, value }: { label: string; value: string }) {
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
			<div className="w-full max-w-130 rounded-2xl border border-[#e5e7eb] bg-white p-6 shadow-xl">{children}</div>
		</div>
	)
}
