import { useState } from 'react'
import { CopyButton } from '@/components/copy-button'
import type { BumpFeeState } from '@/domain/admin-wallet/hooks/use-bump-fee'
import { isValidBumpRate } from '@/domain/admin-wallet/model/bump-fee-rate'
import { formatAdminWalletError } from '@/domain/admin-wallet/model/format-admin-wallet-error'
import { truncTxid } from '@/domain/admin-wallet/model/trunc-txid'
import {
	FEE_RATE_STEP_SAT_PER_KVB,
	feeSats,
	formatSatPerVb,
	parseSatPerVb,
} from '@/domain/fee-selection/model/fee-rate'

export type BumpFeeFormProps = {
	/** Display label of the rate being replaced, e.g. "1.0 sat/vB". */
	currentFeeRateLabel: string
	minBumpSatPerKvb: number
	suggestedSatPerKvb: number
	maxSatPerKvb: number
	vsizeVbytes: number
	state: BumpFeeState
	onConfirm(satPerKvb: number): void
	onClose(): void
}

export function BumpFeeForm({
	currentFeeRateLabel,
	minBumpSatPerKvb,
	suggestedSatPerKvb,
	maxSatPerKvb,
	vsizeVbytes,
	state,
	onConfirm,
	onClose,
}: BumpFeeFormProps) {
	const [rateInput, setRateInput] = useState(() => formatSatPerVb(suggestedSatPerKvb))

	if (state.status === 'success') {
		return (
			<div
				className="mb-2 rounded-xl border border-[#a7f3d0] bg-[#ecfdf5] px-3 py-2.5"
				data-testid="e2e-wallet-bump-success"
			>
				<p className="m-0 text-[12px] font-medium text-[#047857]">Replacement broadcast</p>
				<p
					className="m-0 mt-1 flex items-center gap-1.5 font-mono text-[12px] text-[#065f46]"
					title={state.result.newTxid}
				>
					{truncTxid(state.result.newTxid)}
					<CopyButton text={state.result.newTxid} variant="icon" />
				</p>
				<button
					type="button"
					onClick={onClose}
					className="mt-2 rounded-lg border border-[#a7f3d0] bg-white px-2.5 py-1 text-[11px] font-medium text-[#047857] transition hover:border-[#047857]"
				>
					Done
				</button>
			</div>
		)
	}

	const parsed = parseSatPerVb(rateInput)
	const isSubmitting = state.status === 'submitting'
	const canConfirm = !isSubmitting && isValidBumpRate(parsed, minBumpSatPerKvb, maxSatPerKvb)
	const estimatedFee = parsed !== null ? feeSats(parsed, vsizeVbytes) : null

	function handleStep(direction: 1 | -1) {
		const current = parsed ?? suggestedSatPerKvb
		const next = Math.max(minBumpSatPerKvb, Math.min(maxSatPerKvb, current + direction * FEE_RATE_STEP_SAT_PER_KVB))
		setRateInput(formatSatPerVb(next))
	}

	return (
		<div
			className="mb-2 rounded-xl border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5"
			data-testid="e2e-wallet-bump-form"
		>
			<div className="flex flex-wrap items-center gap-3">
				<label htmlFor="bump-fee-rate" className="text-[12px] font-medium text-[#374151]">
					New rate
				</label>
				<div className="flex items-center gap-1.5">
					<button
						type="button"
						aria-label="Decrease fee rate"
						onClick={() => handleStep(-1)}
						disabled={isSubmitting}
						className="flex h-8 w-8 items-center justify-center rounded-lg border border-[#e5e7eb] bg-white text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827] disabled:cursor-not-allowed disabled:text-[#d1d5db]"
					>
						−
					</button>
					<div className="flex items-center overflow-hidden rounded-lg border border-[#e5e7eb] bg-white transition focus-within:border-[#111827]">
						<input
							id="bump-fee-rate"
							type="text"
							inputMode="decimal"
							autoFocus
							value={rateInput}
							onChange={(e) => setRateInput(e.target.value)}
							disabled={isSubmitting}
							data-testid="e2e-wallet-bump-rate-input"
							className="w-16 border-0 bg-transparent px-2.5 py-1.5 text-center text-[13px] font-medium text-[#111827] focus:outline-none"
						/>
						<span className="pr-2.5 text-[12px] text-[#9ca3af]">sat/vB</span>
					</div>
					<button
						type="button"
						aria-label="Increase fee rate"
						onClick={() => handleStep(1)}
						disabled={isSubmitting}
						className="flex h-8 w-8 items-center justify-center rounded-lg border border-[#e5e7eb] bg-white text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827] disabled:cursor-not-allowed disabled:text-[#d1d5db]"
					>
						+
					</button>
				</div>
			</div>

			<p className="m-0 mt-1.5 text-[11px] text-[#9ca3af]">
				Current {currentFeeRateLabel} · min {formatSatPerVb(minBumpSatPerKvb)} · max {formatSatPerVb(maxSatPerKvb)}
				{estimatedFee !== null ? ` · new fee ~${estimatedFee.toLocaleString()} sats` : ''}
			</p>

			{state.status === 'error' && (
				<p className="m-0 mt-1.5 text-[12px] text-[#ef4444]" data-testid="e2e-wallet-bump-error">
					{formatAdminWalletError(state.error).body}
				</p>
			)}

			<div className="mt-2 flex items-center gap-2">
				<button
					type="button"
					onClick={() => parsed !== null && onConfirm(parsed)}
					disabled={!canConfirm}
					data-testid="e2e-wallet-bump-confirm"
					className={`rounded-lg px-3 py-1.5 text-[12px] font-medium transition ${
						canConfirm ? 'bg-[#111827] text-white hover:bg-[#1f2937]' : 'cursor-not-allowed bg-[#f3f4f6] text-[#9ca3af]'
					}`}
				>
					{isSubmitting ? 'Broadcasting…' : 'Confirm bump'}
				</button>
				<button
					type="button"
					onClick={onClose}
					disabled={isSubmitting}
					className="rounded-lg border border-[#e5e7eb] bg-white px-3 py-1.5 text-[12px] font-medium text-[#374151] transition hover:border-[#d1d5db] disabled:cursor-not-allowed disabled:text-[#d1d5db]"
				>
					Cancel
				</button>
			</div>
		</div>
	)
}
