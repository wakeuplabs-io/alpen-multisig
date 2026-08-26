import { useEffect, useState } from 'react'
import { buildDefcon1ActionHex } from '@/api/action-builder'

/**
 * Defcon 1 carries no payload, so its action hex is a constant. Resolving it here — rather
 * than waiting for the preview step, as the payload-carrying actions do — is what lets the
 * form render the canonical signing message while the signer is still filling it in.
 */
export function useDefcon1ActionHex(): string | null {
	const [actionHex, setActionHex] = useState<string | null>(null)

	useEffect(() => {
		let cancelled = false
		void buildDefcon1ActionHex().then((result) => {
			if (cancelled || !result.ok) return
			setActionHex(result.data.actionHex)
		})
		return () => {
			cancelled = true
		}
	}, [])

	return actionHex
}
