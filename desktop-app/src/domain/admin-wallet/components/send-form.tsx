import { useState } from 'react'
import { useSend } from '@/domain/admin-wallet/hooks/use-send'
import { formatAdminWalletError } from '@/domain/admin-wallet/model/format-admin-wallet-error'
import { formatBtcFromSats } from '@/domain/admin-wallet/model/format-btc-from-sats'
import { canConfirmSend, parseAmountSats } from '@/domain/admin-wallet/model/send-validation'
import { formatSatPerVb } from '@/domain/fee-selection/model/fee-rate'
import { useFeePresets } from '@/domain/fee-selection/hooks/use-fee-presets'
import { SendResultCard } from './send-result-card'

export type SendFormProps = {
	/** Watch-only sessions see the form disabled ("Hardware wallet required to sign"). */
	isWatchOnly: boolean
	/** Returns to the panel root (also wired to the Done button on success). */
	onBack(): void
	/** Called after a successful send so the panel re-syncs and re-reads. */
	onAfterSend(): void
}

/**
 * Phase 6 (PRD §4.3.5) — Send BTC form, P6.1 walking skeleton: destination,
 * amount in sats, default "next block" (Fast) fee rate, Confirm → txid.
 * P6.2 adds inline destination validation, P6.3 the fee control + Max +
 * estimate, P6.4 the final confirm-gate contract.
 *
 * On submit failure the field values are retained for retry or back-out
 * (PRD §4.3.5.5.1) — submission state lives in `useSend`, fields stay local.
 */
export function SendForm({ isWatchOnly, onBack, onAfterSend }: SendFormProps) {
	const [address, setAddress] = useState('')
	const [amountInput, setAmountInput] = useState('')
	const feePresets = useFeePresets()
	const { state, send, reset } = useSend()

	if (isWatchOnly) {
		return (
			<div className="rounded-xl border border-[#e5e7eb] bg-[#f9fafb] px-4 py-3" data-testid="e2e-wallet-send-form">
				<p className="m-0 text-[13px] font-medium text-[#374151]">Hardware wallet required to sign</p>
				<p className="m-0 mt-1 text-[12px] text-[#6b7280]">
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
					setAddress('')
					setAmountInput('')
					onAfterSend()
					onBack()
				}}
			/>
		)
	}

	const amountSats = parseAmountSats(amountInput)
	const isSubmitting = state.status === 'submitting'
	// PRD §4.3.5.3: the default rate is the node's "next block" estimate — the Fast preset.
	const fastRateSatPerKvb = feePresets.status === 'ready' ? feePresets.presets.fast.satPerKvb : null
	const canConfirm = canConfirmSend({
		hasDestination: address.trim() !== '',
		amountSats,
		isFeeReady: fastRateSatPerKvb !== null,
		isSubmitting,
	})

	function handleConfirm() {
		if (fastRateSatPerKvb === null || amountSats === null) return
		void send({ address: address.trim(), amountSats, feeRateSatPerKvb: fastRateSatPerKvb })
	}

	return (
		<div className="flex flex-col gap-3" data-testid="e2e-wallet-send-form">
			<div>
				<label htmlFor="send-address" className="text-[12px] font-medium text-[#374151]">
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
					data-testid="e2e-wallet-send-address-input"
					className="mt-1 w-full rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-2 font-mono text-[12px] text-[#111827] transition focus:border-[#111827] focus:outline-none disabled:bg-[#f9fafb] disabled:text-[#9ca3af]"
				/>
			</div>

			<div>
				<label htmlFor="send-amount" className="text-[12px] font-medium text-[#374151]">
					Amount
				</label>
				<div className="mt-1 flex items-center overflow-hidden rounded-lg border border-[#e5e7eb] bg-white transition focus-within:border-[#111827]">
					<input
						id="send-amount"
						type="text"
						inputMode="numeric"
						placeholder="0"
						value={amountInput}
						onChange={(e) => setAmountInput(e.target.value)}
						disabled={isSubmitting}
						data-testid="e2e-wallet-send-amount-input"
						className="w-full border-0 bg-transparent px-2.5 py-2 text-[13px] font-medium text-[#111827] focus:outline-none disabled:text-[#9ca3af]"
					/>
					<span className="pr-2.5 text-[12px] text-[#9ca3af]">sats</span>
				</div>
				{amountSats !== null && amountSats > 0 && (
					<p className="m-0 mt-1 text-[11px] text-[#9ca3af]">{formatBtcFromSats(amountSats)} BTC</p>
				)}
			</div>

			<p className="m-0 text-[11px] text-[#9ca3af]" data-testid="e2e-wallet-send-fee-control">
				{fastRateSatPerKvb !== null
					? `Network fee rate · ${formatSatPerVb(fastRateSatPerKvb)} sat/vB (next block)`
					: feePresets.status === 'error'
						? 'Fee estimate unavailable — try again shortly.'
						: 'Loading fee estimate…'}
			</p>

			{state.status === 'error' && (
				<p className="m-0 text-[12px] text-[#ef4444]" data-testid="e2e-wallet-send-error">
					{formatAdminWalletError(state.error).body}
				</p>
			)}

			<div className="flex items-center gap-2">
				<button
					type="button"
					onClick={handleConfirm}
					disabled={!canConfirm}
					data-testid="e2e-wallet-send-confirm"
					className={`rounded-lg px-3 py-1.5 text-[12px] font-medium transition ${
						canConfirm ? 'bg-[#111827] text-white hover:bg-[#1f2937]' : 'cursor-not-allowed bg-[#f3f4f6] text-[#9ca3af]'
					}`}
				>
					{isSubmitting ? 'Sending…' : 'Confirm'}
				</button>
				<button
					type="button"
					onClick={onBack}
					disabled={isSubmitting}
					className="rounded-lg border border-[#e5e7eb] bg-white px-3 py-1.5 text-[12px] font-medium text-[#374151] transition hover:border-[#d1d5db] disabled:cursor-not-allowed disabled:text-[#d1d5db]"
				>
					Back
				</button>
			</div>
		</div>
	)
}
