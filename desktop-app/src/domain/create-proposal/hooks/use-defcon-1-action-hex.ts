import { useEffect, useState } from 'react'
import { buildDefcon1ActionHex } from '@/api/action-builder'

export type Defcon1ActionHex = {
	actionHex: string | null
	error: string | null
}

/**
 * Defcon 1 carries no payload, so its action hex is a constant. Resolving it here — rather
 * than waiting for the preview step, as the payload-carrying actions do — is what lets the
 * form render the canonical signing message while the signer is still filling it in.
 *
 * The failure is returned, not swallowed: a signing message that never resolved must read as
 * broken, never as "you have not typed a sequence number yet".
 */
export function useDefcon1ActionHex(): Defcon1ActionHex {
	const [state, setState] = useState<Defcon1ActionHex>({ actionHex: null, error: null })

	useEffect(() => {
		let cancelled = false
		void buildDefcon1ActionHex().then((result) => {
			if (cancelled) return
			setState(result.ok ? { actionHex: result.data.actionHex, error: null } : { actionHex: null, error: result.error })
		})
		return () => {
			cancelled = true
		}
	}, [])

	return state
}
