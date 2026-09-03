import { useEffect, useState } from 'react'
import { buildDefcon1ActionHex, buildDefcon3ActionHex } from '@/api/action-builder'
import type { DefconLevel } from '@/lib/defcon-copy'

export type DefconActionHex = {
	actionHex: string | null
	error: string | null
}

/**
 * Both Defcon levers carry no payload, so their action hex is a constant. Resolving it here —
 * rather than waiting for the preview step, as the payload-carrying actions do — is what lets the
 * form render the canonical signing message while the signer is still filling it in.
 *
 * The failure is returned, not swallowed: a signing message that never resolved must read as
 * broken, never as "you have not typed a sequence number yet".
 *
 * The state is cleared before each resolve. Keeping the previous level's hex across a switch would
 * render one lever's signing message under the other's heading, and the pairing guard downstream
 * matches the message against the hex, not the hex against the level — so it would not catch it.
 */
export function useDefconActionHex(level: DefconLevel): DefconActionHex {
	const [state, setState] = useState<DefconActionHex>({ actionHex: null, error: null })

	useEffect(() => {
		let cancelled = false
		setState({ actionHex: null, error: null })
		const build = level === 'defcon_1' ? buildDefcon1ActionHex : buildDefcon3ActionHex
		void build().then((result) => {
			if (cancelled) return
			setState(result.ok ? { actionHex: result.data.actionHex, error: null } : { actionHex: null, error: result.error })
		})
		return () => {
			cancelled = true
		}
	}, [level])

	return state
}
