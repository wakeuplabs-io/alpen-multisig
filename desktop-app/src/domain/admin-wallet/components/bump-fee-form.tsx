import { useState } from 'react'
import { CopyButton } from '@/components/copy-button'
import type { BumpFeeState } from '@/domain/admin-wallet/hooks/use-bump-fee'
import { isValidBumpRate } from '@/domain/admin-wallet/model/bump-fee-rate'
import { formatAdminWalletError } from '@/domain/admin-wallet/model/format-admin-wallet-error'
import { truncTxid } from '@/domain/admin-wallet/model/trunc-txid'
import type { BumpMethodDto } from '@/domain/admin-wallet/model/types'
import {
	FEE_RATE_STEP_SAT_PER_KVB,
	feeSats,
	formatSatPerVb,
	parseSatPerVb,
} from '@/domain/fee-selection/model/fee-rate'

/**
 * Size of the common CPFP child: one P2TR input (the reveal's change) and one P2TR
 * output. The backend no longer carries this as a constant — it sizes the child from the
 * wallet descriptor — but the figure is the same whenever the anchor funds the child on
 * its own, which is the ordinary case.
 *
 * When the anchor cannot cover fee and output the backend pulls in another coin, and the
 * real child is a full input larger (#431). The estimate below is then low, and the
 * confirmed result reports the true figure. Treated as an estimate on purpose: the exact
 * number depends on a coin selection the UI does not run.
 */
const CPFP_CHILD_VSIZE_EST_VBYTES = 111

export type BumpFeeFormProps = {
	/** 'rbf' replaces the tx; 'cpfp' broadcasts a child that accelerates the package. */
	method: BumpMethodDto
	/** Display label of the rate being bumped (package rate for CPFP), e.g. "1.0 sat/vB". */
	currentFeeRateLabel: string
	/** Fee already paid (package fee for CPFP) — drives the CPFP child-fee estimate. */
	currentFeeSats: number | null
	minBumpSatPerKvb: number
	suggestedSatPerKvb: number
	maxSatPerKvb: number
	vsizeVbytes: number
	/** Target txid being bumped (for F-011 summary). */
	targetTxid: string
	state: BumpFeeState
	onConfirm(satPerKvb: number): void
	onClose(): void
}

function estimatedCostSats(
	method: BumpMethodDto,
	satPerKvb: number,
	vsizeVbytes: number,
	currentFeeSats: number | null,
): number | null {
	if (method === 'rbf') return feeSats(satPerKvb, vsizeVbytes)
	if (currentFeeSats === null) return null
	return Math.max(0, feeSats(satPerKvb, vsizeVbytes + CPFP_CHILD_VSIZE_EST_VBYTES) - currentFeeSats)
}

export function BumpFeeForm({
	method,
	currentFeeRateLabel,
	currentFeeSats,
	minBumpSatPerKvb,
	suggestedSatPerKvb,
	maxSatPerKvb,
	vsizeVbytes,
	targetTxid,
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
				<p className="m-0 text-label font-medium text-[#047857]">
					{state.result.method === 'cpfp' ? 'Acceleration sent (CPFP child)' : 'Replacement sent'}
				</p>
				{state.result.method === 'cpfp' && (
					<p className="m-0 mt-0.5 text-mono-sm text-[#065f46]">
						A child transaction now pays for the whole package — the commit and reveal stay untouched.
					</p>
				)}
				<p
					className="m-0 mt-1 flex items-center gap-1.5 font-mono text-label text-[#065f46]"
					title={state.result.newTxid}
				>
					{truncTxid(state.result.newTxid)}
					<CopyButton text={state.result.newTxid} variant="icon" />
				</p>
				<button
					type="button"
					onClick={onClose}
					className="mt-2 rounded-lg border border-[#a7f3d0] bg-white px-2.5 py-1 text-mono-sm font-medium text-[#047857] transition hover:border-[#047857]"
				>
					Done
				</button>
			</div>
		)
	}

	const parsed = parseSatPerVb(rateInput)
	const isSubmitting = state.status === 'submitting'
	const canConfirm = !isSubmitting && isValidBumpRate(parsed, minBumpSatPerKvb, maxSatPerKvb)
	const estimatedFee = parsed !== null ? estimatedCostSats(method, parsed, vsizeVbytes, currentFeeSats) : null
	const estimateNoun = method === 'cpfp' ? 'child fee' : 'new fee'

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
				<label htmlFor="bump-fee-rate" className="text-label font-medium text-[#374151]">
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
							className="w-16 border-0 bg-transparent px-2.5 py-1.5 text-center text-body-sm font-medium text-[#111827] focus:outline-none"
						/>
						<span className="pr-2.5 text-label text-[#9ca3af]">sat/vB</span>
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

			{method === 'cpfp' && (
				<p className="m-0 mt-1.5 text-mono-sm text-[#6b7280]">
					Accelerates via a child transaction (CPFP) — the new rate applies to the whole commit+reveal package.
				</p>
			)}

			<p className="m-0 mt-1.5 text-mono-sm text-[#9ca3af]">
				Current {currentFeeRateLabel} · min {formatSatPerVb(minBumpSatPerKvb)} · max {formatSatPerVb(maxSatPerKvb)}
				{estimatedFee !== null ? ` · ${estimateNoun} ~${estimatedFee.toLocaleString()} sats` : ''}
			</p>

			{state.status === 'error' && (
				<p className="m-0 mt-1.5 text-label text-danger" data-testid="e2e-wallet-bump-error">
					{formatAdminWalletError(state.error).body}
				</p>
			)}

			{/* F-011: Lightweight summary before confirming — shows method, target txid, and estimated fee */}
			<div className="mt-2 rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-1.5">
				<p className="m-0 text-mono-sm text-[#6b7280]">
					<span className="font-medium text-[#374151]">{method === 'cpfp' ? 'CPFP' : 'RBF'}</span>
					{' · '}
					<span className="font-mono" title={targetTxid}>
						tx {truncTxid(targetTxid)}
					</span>
					{estimatedFee !== null && (
						<>
							{' · '}
							<span className="font-medium text-[#111827]">~{estimatedFee.toLocaleString()} sats</span>
						</>
					)}
				</p>
			</div>

			<div className="mt-2 flex items-center gap-2">
				<button
					type="button"
					onClick={() => parsed !== null && onConfirm(parsed)}
					disabled={!canConfirm}
					data-testid="e2e-wallet-bump-confirm"
					className={`rounded-lg px-3 py-1.5 text-label font-medium transition ${
						canConfirm ? 'bg-[#111827] text-white hover:bg-[#1f2937]' : 'cursor-not-allowed bg-[#f3f4f6] text-[#9ca3af]'
					}`}
				>
					{isSubmitting ? 'Sending…' : 'Confirm bump'}
				</button>
				<button
					type="button"
					onClick={onClose}
					disabled={isSubmitting}
					className="rounded-lg border border-[#e5e7eb] bg-white px-3 py-1.5 text-label font-medium text-[#374151] transition hover:border-[#d1d5db] disabled:cursor-not-allowed disabled:text-[#d1d5db]"
				>
					Cancel
				</button>
			</div>
		</div>
	)
}
