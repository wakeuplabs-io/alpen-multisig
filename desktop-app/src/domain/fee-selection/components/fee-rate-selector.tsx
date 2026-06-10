import { useState } from 'react'
import type { ComponentType } from 'react'
import type { FeeRates } from '@/api/fee-rates'
import { AlertTriangleIcon, BoltIcon, CheckWhiteIcon, GaugeIcon, HourglassIcon, SlidersIcon } from '@/assets/icons'
import type { IconProps } from '@/assets/icons'
import {
	FEE_RATE_STEP_SAT_PER_KVB,
	SAT_PER_VB_TO_KVB,
	clampCustomRate,
	feeSats,
	formatSatPerVb,
	parseSatPerVb,
	totalFeeSats,
} from '../model/fee-rate'
import type { FeePresetKey, FeeSelection } from '../model/fee-rate'

type Props = {
	presets: FeeRates
	selection: FeeSelection
	onSelectPreset: (preset: FeePresetKey) => void
	onSetCustomRate: (satPerKvb: number) => void
}

const PRESET_OPTIONS: { key: FeePresetKey; label: string; Icon: ComponentType<IconProps> }[] = [
	{ key: 'slow', label: 'Slow', Icon: HourglassIcon },
	{ key: 'medium', label: 'Medium', Icon: GaugeIcon },
	{ key: 'fast', label: 'Fast', Icon: BoltIcon },
]

function ActiveCheckBadge() {
	return (
		<span
			className="absolute right-1.5 top-1.5 flex h-4 w-4 items-center justify-center rounded-full bg-[#111827]"
			aria-hidden="true"
		>
			<CheckWhiteIcon width={9} height={9} />
		</span>
	)
}

function FeeBreakdownRow({ label, sats }: { label: string; sats: number }) {
	return (
		<div className="flex items-center justify-between px-3 py-2">
			<dt className="text-[#6b7280]">{label}</dt>
			<dd className="m-0 font-medium text-[#111827]">{sats.toLocaleString()} sats</dd>
		</div>
	)
}

export function FeeRateSelector({ presets, selection, onSelectPreset, onSetCustomRate }: Props) {
	const [customInput, setCustomInput] = useState('')

	const isCustom = selection.kind === 'custom'
	const activeSatPerKvb = isCustom ? selection.satPerKvb : presets[selection.preset].satPerKvb
	const commitFee = feeSats(activeSatPerKvb, presets.commitVbytesEstimate)
	const revealFee = feeSats(activeSatPerKvb, presets.revealVbytes)
	const totalFee = totalFeeSats(activeSatPerKvb, presets)

	function handleCustomChange(raw: string) {
		setCustomInput(raw)
		const parsed = parseSatPerVb(raw)
		if (parsed !== null) {
			onSetCustomRate(parsed)
		}
	}

	function handleCustomBlur() {
		const parsed = parseSatPerVb(customInput)
		if (parsed !== null) {
			setCustomInput(formatSatPerVb(clampCustomRate(parsed, presets)))
		}
	}

	function handleSelectCustom() {
		const val = parseSatPerVb(customInput) ?? activeSatPerKvb
		setCustomInput(formatSatPerVb(val))
		onSetCustomRate(val)
	}

	return (
		<div className="flex flex-col gap-3">
			<div role="radiogroup" aria-label="Network fee rate" className="grid grid-cols-4 gap-2">
				{PRESET_OPTIONS.map(({ key, label, Icon }) => {
					const rate = presets[key]
					const isActive = selection.kind === 'preset' && selection.preset === key
					return (
						<button
							key={key}
							type="button"
							role="radio"
							aria-checked={isActive}
							onClick={() => onSelectPreset(key)}
							className={`relative flex flex-col items-center gap-0.5 rounded-xl border px-2 py-3 transition ${
								isActive
									? 'border-[#111827] bg-[#fafafa] shadow-sm'
									: 'border-[#e5e7eb] bg-white hover:border-[#d1d5db]'
							}`}
						>
							{isActive && <ActiveCheckBadge />}
							<Icon width={18} height={18} className={isActive ? 'text-[#111827]' : 'text-[#9ca3af]'} />
							<span className={`mt-1 text-[13px] font-medium ${isActive ? 'text-[#111827]' : 'text-[#374151]'}`}>
								{label}
							</span>
							<span className={`text-[12px] ${isActive ? 'text-[#374151]' : 'text-[#6b7280]'}`}>
								{formatSatPerVb(rate.satPerKvb)} sat/vB
							</span>
							<span className="text-[11px] text-[#9ca3af]">
								~{rate.targetBlocks} block{rate.targetBlocks !== 1 ? 's' : ''}
							</span>
						</button>
					)
				})}

				<button
					type="button"
					role="radio"
					aria-checked={isCustom}
					onClick={handleSelectCustom}
					className={`relative flex flex-col items-center gap-0.5 rounded-xl border px-2 py-3 transition ${
						isCustom ? 'border-[#111827] bg-[#fafafa] shadow-sm' : 'border-[#e5e7eb] bg-white hover:border-[#d1d5db]'
					}`}
				>
					{isCustom && <ActiveCheckBadge />}
					<SlidersIcon width={18} height={18} className={isCustom ? 'text-[#111827]' : 'text-[#9ca3af]'} />
					<span className={`mt-1 text-[13px] font-medium ${isCustom ? 'text-[#111827]' : 'text-[#374151]'}`}>
						Custom
					</span>
					<span className={`text-[12px] ${isCustom ? 'text-[#374151]' : 'text-[#6b7280]'}`}>
						{isCustom ? `${formatSatPerVb(activeSatPerKvb)} sat/vB` : '—'}
					</span>
					<span className="text-[11px] text-[#9ca3af]">manual rate</span>
				</button>
			</div>

			{isCustom && (
				<div className="flex flex-wrap items-center gap-3 rounded-xl border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
					<label htmlFor="custom-fee-rate" className="text-[12px] font-medium text-[#374151]">
						Custom rate
					</label>
					<div className="flex items-center overflow-hidden rounded-lg border border-[#e5e7eb] bg-white transition focus-within:border-[#111827]">
						<input
							id="custom-fee-rate"
							type="number"
							autoFocus
							min={presets.minRelaySatPerKvb / SAT_PER_VB_TO_KVB}
							max={presets.maxSatPerKvb / SAT_PER_VB_TO_KVB}
							step={FEE_RATE_STEP_SAT_PER_KVB / SAT_PER_VB_TO_KVB}
							placeholder={formatSatPerVb(activeSatPerKvb)}
							value={customInput}
							onChange={(e) => handleCustomChange(e.target.value)}
							onBlur={handleCustomBlur}
							className="w-24 border-0 bg-transparent px-3 py-1.5 text-[13px] text-[#111827] focus:outline-none"
						/>
						<span className="pr-3 text-[12px] text-[#9ca3af]">sat/vB</span>
					</div>
					<span className="ml-auto text-[11px] text-[#9ca3af]">
						min {formatSatPerVb(presets.minRelaySatPerKvb)} · max {formatSatPerVb(presets.maxSatPerKvb)} sat/vB
					</span>
				</div>
			)}

			{presets.source === 'fallback' && (
				<div className="flex items-start gap-2 rounded-lg border border-[#fde68a] bg-[#fffbeb] px-3 py-2.5">
					<AlertTriangleIcon width={14} height={14} className="mt-0.5 shrink-0 text-[#d97706]" />
					<p className="m-0 text-[12px] leading-relaxed text-[#92400e]">
						Live fee estimates are unavailable — these rates are a static fallback from the minimum relay fee. Review
						before broadcasting.
					</p>
				</div>
			)}

			<div className="overflow-hidden rounded-lg border border-[#e5e7eb]">
				<dl className="m-0 divide-y divide-[#f3f4f6] bg-white text-[12px]">
					<FeeBreakdownRow label="Commit fee (est.)" sats={commitFee} />
					<FeeBreakdownRow label="Reveal fee" sats={revealFee} />
					<FeeBreakdownRow label="Dust output (commit)" sats={presets.commitDustSats} />
				</dl>
				<div className="flex items-center justify-between border-t border-[#e5e7eb] bg-[#f9fafb] px-3 py-2">
					<span className="text-[11px] font-semibold uppercase tracking-wider text-[#9ca3af]">Estimated total fee</span>
					<span className="text-[13px] font-semibold text-[#111827]">{totalFee.toLocaleString()} sats</span>
				</div>
			</div>
		</div>
	)
}
