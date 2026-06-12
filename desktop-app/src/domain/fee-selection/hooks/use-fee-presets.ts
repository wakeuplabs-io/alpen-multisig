import { useCallback, useEffect, useState } from 'react'
import { estimateFeeRates, type FeeRates } from '@/api/fee-rates'
import { clampCustomRate, resolveRate } from '../model/fee-rate'
import type { FeePresetKey, FeeSelection } from '../model/fee-rate'

export type FeePresetsState =
	| { status: 'loading' }
	| { status: 'error'; message: string }
	| {
			status: 'ready'
			presets: FeeRates
			selection: FeeSelection
			satPerKvb: number
			setPreset: (preset: FeePresetKey) => void
			setCustomRate: (satPerKvb: number) => void
	  }

/**
 * @param initialSelection Governance broadcast defaults to Medium; the wallet
 * Send form passes Fast — PRD §4.3.5.3 requires the "next block" rate as the
 * default there.
 */
export function useFeePresets(initialSelection: FeeSelection = { kind: 'preset', preset: 'medium' }): FeePresetsState {
	const [presets, setPresets] = useState<FeeRates | null>(null)
	const [error, setError] = useState<string | null>(null)
	const [selection, setSelection] = useState<FeeSelection>(initialSelection)

	useEffect(() => {
		let active = true
		estimateFeeRates().then((result) => {
			if (!active) return
			if (result.ok) {
				setPresets(result.data)
			} else {
				setError(result.error)
			}
		})
		return () => {
			active = false
		}
	}, [])

	const setPreset = useCallback((preset: FeePresetKey) => {
		setSelection({ kind: 'preset', preset })
	}, [])

	const setCustomRate = useCallback(
		(satPerKvb: number) => {
			if (presets === null) return
			setSelection({ kind: 'custom', satPerKvb: clampCustomRate(satPerKvb, presets) })
		},
		[presets],
	)

	if (error !== null) return { status: 'error', message: error }
	if (presets === null) return { status: 'loading' }

	return {
		status: 'ready',
		presets,
		selection,
		satPerKvb: resolveRate(selection, presets),
		setPreset,
		setCustomRate,
	}
}
