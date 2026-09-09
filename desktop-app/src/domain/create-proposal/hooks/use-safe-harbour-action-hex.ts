import { useEffect, useState } from 'react'
import { buildSafeHarbourAddressUpdateHex } from '@/api/action-builder'

export type SafeHarbourActionHex = {
	actionHex: string | null
	error: string | null
}

/**
 * Resolves the action hex for a safe harbour rotation, so the form can render the canonical signing
 * message while the signer is still filling it in — the same thing `use-defcon-action-hex.ts` does,
 * with one difference: the hex depends on what was typed rather than being a constant.
 *
 * That difference is why the resolve is keyed on the address and not on every keystroke: the
 * builder is an IPC round trip, and while the address is invalid the field's own error is what the
 * signer needs to see anyway. `address` here is expected to be the debounced-by-validity value —
 * callers pass an empty string until they have something worth resolving.
 *
 * Two rules taken from the Defcon hook verbatim, for the same reasons:
 *
 * - the state is cleared before each resolve, so a message resolved for one destination is never
 *   left standing under another;
 * - the failure is returned rather than swallowed, so a hex that never resolved reads as broken
 *   instead of as "you have not finished typing".
 */
export function useSafeHarbourActionHex(address: string): SafeHarbourActionHex {
	const [state, setState] = useState<SafeHarbourActionHex>({ actionHex: null, error: null })

	useEffect(() => {
		let cancelled = false
		setState({ actionHex: null, error: null })
		if (address.trim().length === 0) return
		void buildSafeHarbourAddressUpdateHex({ address: address.trim() }).then((result) => {
			if (cancelled) return
			setState(result.ok ? { actionHex: result.data.actionHex, error: null } : { actionHex: null, error: result.error })
		})
		return () => {
			cancelled = true
		}
	}, [address])

	return state
}
