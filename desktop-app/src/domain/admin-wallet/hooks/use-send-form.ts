import { useCallback, useEffect, useRef, useState } from 'react'
import { estimateSend, validateSendAddress } from '@/api/admin-wallet'
import type { AdminWalletError, SendAddressInvalidReason, SendEstimateDto, SendInput } from '@/api/admin-wallet'
import { useFeePresets } from '@/domain/fee-selection/hooks/use-fee-presets'
import type { FeePresetsState } from '@/domain/fee-selection/hooks/use-fee-presets'
import { parseAmountSats } from '../model/send-validation'
import { parseAdminWalletError } from './parse-admin-wallet-error'

/** Debounce window for the destination validation IPC (spec §6.3). */
export const VALIDATE_DEBOUNCE_MS = 300

/** Debounce window for the estimate dry-run IPC (spec §6.3). */
export const ESTIMATE_DEBOUNCE_MS = 300

/**
 * Destination field state machine (Phase 6 P6.2, PRD §4.3.5.1).
 * `unavailable` covers a failing validation IPC (no session / transport):
 * Confirm stays blocked with neutral copy — never silently "valid".
 */
export type DestinationState =
	| { status: 'empty' }
	| { status: 'validating'; address: string }
	| { status: 'valid'; address: string }
	| { status: 'invalid'; address: string; reason: SendAddressInvalidReason; expectedNetwork: string }
	| { status: 'unavailable'; address: string }

/**
 * Estimate dry-run state (Phase 6 P6.3, PRD §4.3.5.2/§4.3.5.3). `error`
 * carries the typed boundary failures (`InsufficientFunds`, `AmountBelowDust`)
 * the form renders before Confirm.
 */
export type EstimateState =
	| { status: 'idle' }
	| { status: 'loading' }
	| { status: 'ready'; estimate: SendEstimateDto }
	| { status: 'error'; error: AdminWalletError }

type UseSendFormReturn = {
	address: string
	setAddress(raw: string): void
	destination: DestinationState
	amountInput: string
	/** Manual amount edit — clears Max mode. */
	setAmountInput(raw: string): void
	/** Parsed sats amount; null while empty/invalid. */
	amountSats: number | null
	/** True while the amount tracks the drain-everything boundary. */
	isMax: boolean
	/** Switches to Max mode; the drain estimate fills (and keeps) the amount. */
	applyMax(): void
	/** Fee presets + selection (initialised to Fast — PRD §4.3.5.3). */
	fee: FeePresetsState
	/** Selected rate in sat/kvB; null until presets load. */
	feeRateSatPerKvb: number | null
	estimate: EstimateState
	/** The exact IPC payload Confirm must submit; null while not confirmable. */
	sendInput: SendInput | null
	/** Clears every field (success → Done). */
	resetFields(): void
}

/**
 * Owns the Send form state (Phase 6): fields, debounced backend-authoritative
 * destination validation (P6.2), fee-preset selection defaulting to the
 * "next block" Fast rate (P6.3, §4.3.5.3), and the debounced estimate dry-run
 * that powers the fee preview, the Max boundary, and the §4.3.5.2
 * insufficient-funds check before Confirm.
 *
 * Max semantics: `applyMax()` pins the amount to the drain boundary; a rate
 * change re-runs the drain estimate and refreshes the amount (the boundary
 * moves with the fee — PRD §4.3.5.2); editing the amount or the destination
 * leaves Max mode. Stale IPC responses are discarded via sequence guards.
 */
export function useSendForm(): UseSendFormReturn {
	const [address, setAddressState] = useState('')
	const [amountInput, setAmountInputState] = useState('')
	const [destination, setDestination] = useState<DestinationState>({ status: 'empty' })
	const [isMax, setIsMax] = useState(false)
	const [estimate, setEstimate] = useState<EstimateState>({ status: 'idle' })
	const fee = useFeePresets({ kind: 'preset', preset: 'fast' })
	const validateSeq = useRef(0)
	const estimateSeq = useRef(0)

	const feeRateSatPerKvb = fee.status === 'ready' ? fee.satPerKvb : null
	const amountSats = parseAmountSats(amountInput)

	const setAddress = useCallback((raw: string) => {
		setAddressState(raw)
		setIsMax(false) // a different destination has a different boundary
	}, [])

	const setAmountInput = useCallback((raw: string) => {
		setAmountInputState(raw)
		setIsMax(false) // manual edit leaves Max mode
	}, [])

	const applyMax = useCallback(() => setIsMax(true), [])

	// ── Destination validation (P6.2) ────────────────────────────────────────
	useEffect(() => {
		const trimmed = address.trim()
		// Every keystroke invalidates any in-flight response.
		const seq = ++validateSeq.current
		if (trimmed === '') {
			setDestination({ status: 'empty' })
			return
		}
		setDestination({ status: 'validating', address: trimmed })
		const timer = setTimeout(() => {
			void validateSendAddress(trimmed).then((result) => {
				if (seq !== validateSeq.current) return // stale — a newer keystroke won
				if (!result.ok) {
					setDestination({ status: 'unavailable', address: trimmed })
					return
				}
				if (result.data.isValid) {
					setDestination({ status: 'valid', address: trimmed })
				} else {
					setDestination({
						status: 'invalid',
						address: trimmed,
						reason: result.data.reason ?? 'invalid-address',
						expectedNetwork: result.data.expectedNetwork,
					})
				}
			})
		}, VALIDATE_DEBOUNCE_MS)
		return () => clearTimeout(timer)
	}, [address])

	// ── Estimate dry-run (P6.3) ──────────────────────────────────────────────
	const hasEstimableAmount = isMax || (amountSats !== null && amountSats > 0)
	const destinationAddress = destination.status === 'valid' ? destination.address : null
	useEffect(() => {
		const seq = ++estimateSeq.current
		if (destinationAddress === null || feeRateSatPerKvb === null || !hasEstimableAmount) {
			setEstimate({ status: 'idle' })
			return
		}
		setEstimate({ status: 'loading' })
		const input: SendInput = isMax
			? { address: destinationAddress, amountSats: 0, feeRateSatPerKvb, drainWallet: true }
			: { address: destinationAddress, amountSats: amountSats ?? 0, feeRateSatPerKvb }
		const timer = setTimeout(() => {
			void estimateSend(input).then((result) => {
				if (seq !== estimateSeq.current) return // stale — inputs changed
				if (!result.ok) {
					setEstimate({ status: 'error', error: parseAdminWalletError(result.error) })
					return
				}
				setEstimate({ status: 'ready', estimate: result.data })
				if (isMax) {
					// The boundary moved (e.g. rate change) — keep the field on it.
					const max = String(result.data.amountSats)
					setAmountInputState((current) => (current === max ? current : max))
				}
			})
		}, ESTIMATE_DEBOUNCE_MS)
		return () => clearTimeout(timer)
	}, [destinationAddress, amountSats, isMax, feeRateSatPerKvb, hasEstimableAmount])

	const resetFields = useCallback(() => {
		setAddressState('')
		setAmountInputState('')
		setDestination({ status: 'empty' })
		setIsMax(false)
		setEstimate({ status: 'idle' })
		validateSeq.current += 1
		estimateSeq.current += 1
	}, [])

	const sendInput: SendInput | null =
		destinationAddress !== null && feeRateSatPerKvb !== null && estimate.status === 'ready'
			? isMax
				? { address: destinationAddress, amountSats: 0, feeRateSatPerKvb, drainWallet: true }
				: amountSats !== null && amountSats > 0
					? { address: destinationAddress, amountSats, feeRateSatPerKvb }
					: null
			: null

	return {
		address,
		setAddress,
		destination,
		amountInput,
		setAmountInput,
		amountSats,
		isMax,
		applyMax,
		fee,
		feeRateSatPerKvb,
		estimate,
		sendInput,
		resetFields,
	}
}
