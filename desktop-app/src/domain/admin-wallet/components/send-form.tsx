import { useSend } from '@/domain/admin-wallet/hooks/use-send'
import { useSendForm } from '@/domain/admin-wallet/hooks/use-send-form'
import { formatAdminWalletError } from '@/domain/admin-wallet/model/format-admin-wallet-error'
import { formatBtcFromSats } from '@/domain/admin-wallet/model/format-btc-from-sats'
import {
	SEND_DESTINATION_UNAVAILABLE_COPY,
	SEND_INSUFFICIENT_FUNDS_COPY,
	formatSendDestinationError,
} from '@/domain/admin-wallet/model/format-send-error'
import { amountInputError, canConfirmSend } from '@/domain/admin-wallet/model/send-validation'
import { formatSatPerVb } from '@/domain/fee-selection/model/fee-rate'
import { SendFeeRateControl } from './send-fee-rate-control'
import { SendResultCard } from './send-result-card'

export type SendFormProps = {
	/** Watch-only sessions see the form disabled ("Hardware wallet required to sign"). */
	isWatchOnly: boolean
	/** Returns to the panel root (also wired to the Done button on success). */
	onBack(): void
	/** Called after a successful send so the panel re-syncs and re-reads. */
	onAfterSend(): void
}

function SummaryRow({ label, value, strong = false }: { label: string; value: string; strong?: boolean }) {
	return (
		<div className="flex items-center justify-between px-3 py-1.5">
			<dt className={`m-0 text-mono-sm ${strong ? 'font-semibold text-[#374151]' : 'text-[#6b7280]'}`}>{label}</dt>
			<dd className={`m-0 text-label ${strong ? 'font-semibold text-[#111827]' : 'font-medium text-[#374151]'}`}>
				{value}
			</dd>
		</div>
	)
}

/**
 * Phase 6 (PRD §4.3.5) — Send BTC form. P6.3: full fee-rate control (presets
 * defaulting to the next-block Fast rate, custom 0.1-step), Max via a drain
 * dry-run, live estimate summary, and the §4.3.5.2 "Insufficient funds"
 * boundary surfaced before Confirm. Confirm enables only when destination,
 * amount, fee, and estimate are all ready (§4.3.5.5).
 *
 * On submit failure the field values are retained for retry or back-out
 * (PRD §4.3.5.5.1) — submission state lives in `useSend`, fields in `useSendForm`.
 */
export function SendForm({ isWatchOnly, onBack, onAfterSend }: SendFormProps) {
	const {
		address,
		setAddress,
		destination,
		amountInput,
		setAmountInput,
		amountSats,
		isMax,
		applyMax,
		fee,
		estimate,
		sendInput,
		resetFields,
	} = useSendForm()
	const { state, send, reset } = useSend()

	if (isWatchOnly) {
		return (
			<div className="rounded-xl border border-[#e5e7eb] bg-[#f9fafb] px-4 py-3" data-testid="e2e-wallet-send-form">
				<p className="m-0 text-body-sm font-medium text-[#374151]">Hardware wallet required to sign</p>
				<p className="m-0 mt-1 text-label text-[#6b7280]">
					This session is watch-only. Connect a hardware wallet to send BTC.
				</p>
			</div>
		)
	}

	if (state.status === 'success') {
		return (
			<SendResultCard
				result={state.result}
				onDone={() => {
					reset()
					resetFields()
					onAfterSend()
					onBack()
				}}
			/>
		)
	}

	const isSubmitting = state.status === 'submitting'
	const destinationError =
		destination.status === 'invalid'
			? formatSendDestinationError(destination.reason, destination.expectedNetwork)
			: destination.status === 'unavailable'
				? SEND_DESTINATION_UNAVAILABLE_COPY
				: null
	// Input-shape errors first (no estimate runs for them), then §4.3.5.2
	// boundary failures from the dry-run.
	const amountError =
		amountInputError(amountInput) ??
		(estimate.status === 'error'
			? estimate.error.type === 'InsufficientFunds'
				? SEND_INSUFFICIENT_FUNDS_COPY
				: formatAdminWalletError(estimate.error).body
			: null)
	const canApplyMax = destination.status === 'valid' && fee.status === 'ready' && !isSubmitting
	const canConfirm = canConfirmSend({
		isDestinationValid: destination.status === 'valid',
		amountSats,
		isMax,
		isFeeReady: fee.status === 'ready',
		isEstimateReady: estimate.status === 'ready',
		isSubmitting,
	})

	function handleConfirm() {
		if (sendInput === null || isSubmitting) return
		void send(sendInput)
	}

	return (
		<div className="flex flex-col gap-4" data-testid="e2e-wallet-send-form">
			<div>
				<label htmlFor="send-address" className="text-label font-medium text-[#374151]">
					Send to
				</label>
				<input
					id="send-address"
					type="text"
					autoFocus
					spellCheck={false}
					placeholder="Bitcoin address"
					value={address}
					onChange={(e) => setAddress(e.target.value)}
					disabled={isSubmitting}
					aria-invalid={destinationError !== null}
					data-testid="e2e-wallet-send-address-input"
					className={`mt-1 w-full rounded-lg border bg-white px-2.5 py-2 font-mono text-label text-[#111827] transition focus:outline-none disabled:bg-[#f9fafb] disabled:text-[#9ca3af] ${
						destinationError !== null
							? 'border-[#ef4444] focus:border-[#ef4444]'
							: 'border-[#e5e7eb] focus:border-[#111827]'
					}`}
				/>
				{destinationError !== null && (
					<p className="m-0 mt-1 text-label text-[#ef4444]" data-testid="e2e-wallet-send-address-error">
						{destinationError}
					</p>
				)}
			</div>

			<div>
				<label htmlFor="send-amount" className="text-label font-medium text-[#374151]">
					Amount
				</label>
				<div
					className={`mt-1 flex items-center overflow-hidden rounded-lg border bg-white transition focus-within:border-[#111827] ${
						amountError !== null ? 'border-[#ef4444]' : 'border-[#e5e7eb]'
					}`}
				>
					<input
						id="send-amount"
						type="text"
						inputMode="numeric"
						placeholder="0"
						value={amountInput}
						onChange={(e) => setAmountInput(e.target.value)}
						disabled={isSubmitting}
						aria-invalid={amountError !== null}
						data-testid="e2e-wallet-send-amount-input"
						className="w-full border-0 bg-transparent px-2.5 py-2 text-body-sm font-medium text-[#111827] focus:outline-none disabled:text-[#9ca3af]"
					/>
					<button
						type="button"
						onClick={applyMax}
						disabled={!canApplyMax}
						title={canApplyMax ? 'Send the maximum amount (balance minus fee)' : 'Enter a valid destination first'}
						data-testid="e2e-wallet-send-max"
						className={`mr-1.5 shrink-0 rounded-md px-2 py-0.5 text-mono-sm font-medium transition ${
							isMax
								? 'bg-[#111827] text-white'
								: canApplyMax
									? 'border border-[#e5e7eb] bg-white text-[#374151] hover:border-[#111827]'
									: 'cursor-not-allowed border border-[#f3f4f6] bg-[#f9fafb] text-[#d1d5db]'
						}`}
					>
						Max
					</button>
					<span className="pr-2.5 text-label text-[#9ca3af]">sats</span>
				</div>
				{amountError !== null ? (
					<p className="m-0 mt-1 text-label text-[#ef4444]" data-testid="e2e-wallet-send-amount-error">
						{amountError}
					</p>
				) : (
					amountSats !== null &&
					amountSats > 0 && (
						<p className="m-0 mt-1 text-mono-sm text-[#9ca3af]">
							{formatBtcFromSats(amountSats)} BTC
							{isMax ? ' · maximum at the selected rate' : ''}
						</p>
					)
				)}
			</div>

			<div>
				<p className="m-0 mb-1.5 text-label font-medium text-[#374151]">Network fee</p>
				{fee.status === 'ready' ? (
					<SendFeeRateControl
						presets={fee.presets}
						selection={fee.selection}
						onSelectPreset={fee.setPreset}
						onSetCustomRate={fee.setCustomRate}
						disabled={isSubmitting}
					/>
				) : (
					<p className="m-0 text-mono-sm text-[#9ca3af]">
						{fee.status === 'error' ? 'Fee estimate unavailable — try again shortly.' : 'Loading fee estimate…'}
					</p>
				)}
			</div>

			{(estimate.status === 'ready' || estimate.status === 'loading') && (
				<div
					className="overflow-hidden rounded-lg border border-[#e5e7eb] bg-[#f9fafb]"
					data-testid="e2e-wallet-send-estimate"
				>
					{estimate.status === 'loading' ? (
						<p className="m-0 px-3 py-2 text-mono-sm text-[#9ca3af]">Estimating…</p>
					) : (
						<dl className="m-0 divide-y divide-[#f3f4f6]">
							<SummaryRow label="Amount" value={`${estimate.estimate.amountSats.toLocaleString()} sats`} />
							<SummaryRow
								label="Network fee"
								value={`${estimate.estimate.feeSats.toLocaleString()} sats · ${formatSatPerVb(estimate.estimate.feeRateSatPerKvb)} sat/vB`}
							/>
							{estimate.estimate.changeSats > 0 && (
								<SummaryRow label="Change" value={`${estimate.estimate.changeSats.toLocaleString()} sats`} />
							)}
							<SummaryRow
								label="Total spent"
								value={`${(estimate.estimate.amountSats + estimate.estimate.feeSats).toLocaleString()} sats`}
								strong
							/>
						</dl>
					)}
				</div>
			)}

			{state.status === 'error' && (
				<p className="m-0 text-label text-[#ef4444]" data-testid="e2e-wallet-send-error">
					{formatAdminWalletError(state.error).body}
				</p>
			)}

			<div className="flex items-center gap-2">
				<button
					type="button"
					onClick={handleConfirm}
					disabled={!canConfirm}
					data-testid="e2e-wallet-send-confirm"
					className={`rounded-lg px-3 py-1.5 text-label font-medium transition ${
						canConfirm ? 'bg-[#111827] text-white hover:bg-[#1f2937]' : 'cursor-not-allowed bg-[#f3f4f6] text-[#9ca3af]'
					}`}
				>
					{isSubmitting ? 'Sending…' : 'Confirm'}
				</button>
				<button
					type="button"
					onClick={onBack}
					disabled={isSubmitting}
					className="rounded-lg border border-[#e5e7eb] bg-white px-3 py-1.5 text-label font-medium text-[#374151] transition hover:border-[#d1d5db] disabled:cursor-not-allowed disabled:text-[#d1d5db]"
				>
					Back
				</button>
			</div>
		</div>
	)
}
