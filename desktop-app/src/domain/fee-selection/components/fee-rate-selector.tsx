import { useState } from 'react'
import type { FeeRates } from '@/api/fee-rates'
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

const PRESET_LABELS: Record<FeePresetKey, string> = {
	slow: 'Slow',
	medium: 'Medium',
	fast: 'Fast',
}

export function FeeRateSelector({ presets, selection, onSelectPreset, onSetCustomRate }: Props) {
	const [customInput, setCustomInput] = useState('')

	const activeSatPerKvb = selection.kind === 'preset' ? presets[selection.preset].satPerKvb : selection.satPerKvb

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
			<div className="flex gap-2">
				{(['slow', 'medium', 'fast'] as FeePresetKey[]).map((preset) => {
					const rate = presets[preset]
					const isActive = selection.kind === 'preset' && selection.preset === preset
					return (
						<button
							key={preset}
							type="button"
							onClick={() => onSelectPreset(preset)}
							className={`flex-1 rounded-lg border px-3 py-2 text-sm transition-colors ${
								isActive
									? 'border-blue-500 bg-blue-50 text-blue-700'
									: 'border-gray-200 bg-white text-gray-700 hover:border-gray-300'
							}`}
						>
							<div className="font-medium">{PRESET_LABELS[preset]}</div>
							<div className="text-xs opacity-75">{formatSatPerVb(rate.satPerKvb)} sat/vB</div>
							<div className="text-xs opacity-60">
								~{rate.targetBlocks} block{rate.targetBlocks !== 1 ? 's' : ''}
							</div>
						</button>
					)
				})}
			</div>

			<div className="flex items-center gap-2">
				<button
					type="button"
					onClick={handleSelectCustom}
					className={`rounded-lg border px-3 py-2 text-sm transition-colors ${
						selection.kind === 'custom'
							? 'border-blue-500 bg-blue-50 text-blue-700'
							: 'border-gray-200 bg-white text-gray-700 hover:border-gray-300'
					}`}
				>
					Custom
				</button>
				<input
					type="number"
					min={presets.minRelaySatPerKvb / SAT_PER_VB_TO_KVB}
					max={presets.maxSatPerKvb / SAT_PER_VB_TO_KVB}
					step={FEE_RATE_STEP_SAT_PER_KVB / SAT_PER_VB_TO_KVB}
					placeholder={formatSatPerVb(activeSatPerKvb)}
					value={customInput}
					onChange={(e) => handleCustomChange(e.target.value)}
					onBlur={handleCustomBlur}
					onFocus={() => setCustomInput(formatSatPerVb(activeSatPerKvb))}
					className="w-28 rounded-lg border border-gray-200 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
				/>
				<span className="text-sm text-gray-500">sat/vB</span>
			</div>

			{presets.source === 'fallback' && (
				<p className="m-0 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-700">
					Live fee estimates are unavailable — these rates are a static fallback from the minimum relay fee. Review
					before broadcasting.
				</p>
			)}

			<div className="text-xs text-gray-500">
				<span>
					Reveal fee: {feeSats(activeSatPerKvb, presets.revealVbytes)} sats
					{' · '}
					Total est.: {totalFeeSats(activeSatPerKvb, presets)} sats
					{' · '}
					Dust: {presets.commitDustSats} sats
				</span>
			</div>
		</div>
	)
}
