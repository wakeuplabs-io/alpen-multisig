import { useState } from 'react'
import type { FeeRates } from '@/api/fee-rates'
import { AlertTriangleIcon } from '@/assets/icons'
import {
	FEE_RATE_STEP_SAT_PER_KVB,
	clampCustomRate,
	formatSatPerVb,
	parseSatPerVb,
} from '@/domain/fee-selection/model/fee-rate'
import type { FeePresetKey, FeeSelection } from '@/domain/fee-selection/model/fee-rate'

export type SendFeeRateControlProps = {
	presets: FeeRates
	selection: FeeSelection
	onSelectPreset(preset: FeePresetKey): void
	onSetCustomRate(satPerKvb: number): void
	disabled: boolean
}

const PRESET_OPTIONS: { key: FeePresetKey; label: string }[] = [
	{ key: 'slow', label: 'Slow' },
	{ key: 'medium', label: 'Medium' },
	{ key: 'fast', label: 'Fast' },
]

/**
 * Compact fee-rate control for the wallet Send form (Phase 6 P6.3, PRD
 * §4.3.5.3): Slow / Medium / Fast presets + Custom with a 0.1 sat/vB stepper,
 * bounds from the presets DTO (never hardcoded). Visually a condensed sibling
 * of the governance `FeeRateSelector` — the commit/reveal breakdown does not
 * apply to a wallet send; full chrome sharing is Phase 9.
 */
export function SendFeeRateControl({
	presets,
	selection,
	onSelectPreset,
	onSetCustomRate,
	disabled,
}: SendFeeRateControlProps) {
	const isCustom = selection.kind === 'custom'
	const activeSatPerKvb = isCustom ? selection.satPerKvb : presets[selection.preset].satPerKvb
	const [customInput, setCustomInput] = useState(() => formatSatPerVb(activeSatPerKvb))

	function handleCustomChange(raw: string) {
		setCustomInput(raw)
		const parsed = parseSatPerVb(raw)
		if (parsed !== null) {
			onSetCustomRate(clampCustomRate(parsed, presets))
		}
	}

	function handleSelectCustom() {
		const val = parseSatPerVb(customInput) ?? activeSatPerKvb
		const clamped = clampCustomRate(val, presets)
		setCustomInput(formatSatPerVb(clamped))
		onSetCustomRate(clamped)
	}

	function handleStepCustom(direction: 1 | -1) {
		const current = parseSatPerVb(customInput) ?? activeSatPerKvb
		const next = clampCustomRate(current + direction * FEE_RATE_STEP_SAT_PER_KVB, presets)
		setCustomInput(formatSatPerVb(next))
		onSetCustomRate(next)
	}

	return (
		<div className="flex flex-col gap-2" data-testid="e2e-wallet-send-fee-control">
			<div role="radiogroup" aria-label="Network fee rate" className="grid grid-cols-4 gap-1.5">
				{PRESET_OPTIONS.map(({ key, label }) => {
					const rate = presets[key]
					const isActive = selection.kind === 'preset' && selection.preset === key
					return (
						<button
							key={key}
							type="button"
							role="radio"
							aria-checked={isActive}
							onClick={() => onSelectPreset(key)}
							disabled={disabled}
							className={`flex flex-col items-center rounded-lg border px-1.5 py-2 transition disabled:cursor-not-allowed disabled:opacity-60 ${
								isActive
									? 'border-[#111827] bg-[#fafafa] shadow-sm'
									: 'border-[#e5e7eb] bg-white hover:border-[#d1d5db]'
							}`}
						>
							<span className={`text-[12px] font-medium ${isActive ? 'text-[#111827]' : 'text-[#374151]'}`}>
								{label}
							</span>
							<span className="text-[11px] text-[#6b7280]">{formatSatPerVb(rate.satPerKvb)}</span>
							<span className="text-[10px] text-[#9ca3af]">
								~{rate.targetBlocks} blk{rate.targetBlocks !== 1 ? 's' : ''}
							</span>
						</button>
					)
				})}
				<button
					type="button"
					role="radio"
					aria-checked={isCustom}
					onClick={handleSelectCustom}
					disabled={disabled}
					className={`flex flex-col items-center rounded-lg border px-1.5 py-2 transition disabled:cursor-not-allowed disabled:opacity-60 ${
						isCustom ? 'border-[#111827] bg-[#fafafa] shadow-sm' : 'border-[#e5e7eb] bg-white hover:border-[#d1d5db]'
					}`}
				>
					<span className={`text-[12px] font-medium ${isCustom ? 'text-[#111827]' : 'text-[#374151]'}`}>Custom</span>
					<span className="text-[11px] text-[#6b7280]">{isCustom ? formatSatPerVb(activeSatPerKvb) : '—'}</span>
					<span className="text-[10px] text-[#9ca3af]">sat/vB</span>
				</button>
			</div>

			{isCustom && (
				<div className="flex flex-wrap items-center gap-2 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-2.5 py-2">
					<div className="flex items-center gap-1.5">
						<button
							type="button"
							aria-label="Decrease fee rate"
							onClick={() => handleStepCustom(-1)}
							disabled={disabled}
							className="flex h-7 w-7 items-center justify-center rounded-lg border border-[#e5e7eb] bg-white text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827] disabled:cursor-not-allowed disabled:text-[#d1d5db]"
						>
							−
						</button>
						<div className="flex items-center overflow-hidden rounded-lg border border-[#e5e7eb] bg-white transition focus-within:border-[#111827]">
							<input
								id="send-custom-fee-rate"
								type="text"
								inputMode="decimal"
								value={customInput}
								onChange={(e) => handleCustomChange(e.target.value)}
								onBlur={handleSelectCustom}
								disabled={disabled}
								aria-label="Custom fee rate in sat/vB"
								className="w-16 border-0 bg-transparent px-2 py-1 text-center text-[12px] font-medium text-[#111827] focus:outline-none"
							/>
							<span className="pr-2 text-[11px] text-[#9ca3af]">sat/vB</span>
						</div>
						<button
							type="button"
							aria-label="Increase fee rate"
							onClick={() => handleStepCustom(1)}
							disabled={disabled}
							className="flex h-7 w-7 items-center justify-center rounded-lg border border-[#e5e7eb] bg-white text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827] disabled:cursor-not-allowed disabled:text-[#d1d5db]"
						>
							+
						</button>
					</div>
					<span className="ml-auto text-[10px] text-[#9ca3af]">
						step 0.1 · min {formatSatPerVb(presets.minRelaySatPerKvb)} · max {formatSatPerVb(presets.maxSatPerKvb)}
					</span>
				</div>
			)}

			{presets.source === 'fallback' && (
				<div className="flex items-start gap-2 rounded-lg border border-[#fde68a] bg-[#fffbeb] px-2.5 py-2">
					<AlertTriangleIcon width={13} height={13} className="mt-0.5 shrink-0 text-[#d97706]" />
					<p className="m-0 text-[11px] leading-relaxed text-[#92400e]">
						Live fee estimates are unavailable — these rates are a static fallback. Review before sending.
					</p>
				</div>
			)}
		</div>
	)
}
