import { useCallback, useEffect, useRef, useState } from 'react'
import { validateSendAddress } from '@/api/admin-wallet'
import type { SendAddressInvalidReason } from '@/api/admin-wallet'
import { parseAmountSats } from '../model/send-validation'

/** Debounce window for the destination validation IPC (spec §6.3). */
export const VALIDATE_DEBOUNCE_MS = 300

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

type UseSendFormReturn = {
	address: string
	setAddress(raw: string): void
	destination: DestinationState
	amountInput: string
	setAmountInput(raw: string): void
	/** Parsed sats amount; null while empty/invalid. */
	amountSats: number | null
	/** Clears every field (success → Done). */
	resetFields(): void
}

/**
 * Owns the Send form field state and the debounced, backend-authoritative
 * destination validation (the frontend has no Bitcoin address parser by
 * design). Stale IPC responses are discarded via a request sequence number;
 * the debounce timer is cleared on every keystroke and on unmount.
 */
export function useSendForm(): UseSendFormReturn {
	const [address, setAddress] = useState('')
	const [amountInput, setAmountInput] = useState('')
	const [destination, setDestination] = useState<DestinationState>({ status: 'empty' })
	const requestSeq = useRef(0)

	useEffect(() => {
		const trimmed = address.trim()
		// Every keystroke invalidates any in-flight response.
		const seq = ++requestSeq.current
		if (trimmed === '') {
			setDestination({ status: 'empty' })
			return
		}
		setDestination({ status: 'validating', address: trimmed })
		const timer = setTimeout(() => {
			void validateSendAddress(trimmed).then((result) => {
				if (seq !== requestSeq.current) return // stale — a newer keystroke won
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

	const resetFields = useCallback(() => {
		setAddress('')
		setAmountInput('')
		setDestination({ status: 'empty' })
		requestSeq.current += 1
	}, [])

	return {
		address,
		setAddress,
		destination,
		amountInput,
		setAmountInput,
		amountSats: parseAmountSats(amountInput),
		resetFields,
	}
}
