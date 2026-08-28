import { useEffect, useState } from 'react'
import { getSafeHarbourStatus } from '@/api/asm-state'

/**
 * Whether the bridge is already in safe harbour, read live from the node.
 *
 * `enabled` is what keeps the read off screens that have no lever answering it — a hook cannot
 * be called conditionally, so the condition lives here.
 *
 * A failed read degrades to `false` — no error surface. The only thing this state drives is a
 * note telling the council what the chain already says; a node that cannot answer must never
 * stand between them and the emergency lever, and an error banner on the dashboard would read
 * as a problem with the proposal list it sits above.
 */
export function useSafeHarbourActivated(enabled = true): boolean {
	const [activated, setActivated] = useState(false)

	useEffect(() => {
		if (!enabled) return
		let cancelled = false
		void getSafeHarbourStatus().then((result) => {
			if (cancelled) return
			setActivated(result.ok && result.data.activated)
		})
		return () => {
			cancelled = true
		}
	}, [enabled])

	return activated
}
