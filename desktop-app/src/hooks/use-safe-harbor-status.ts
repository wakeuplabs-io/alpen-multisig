import { useEffect, useState } from 'react'
import { getSafeHarborStatus, type SafeHarborStatus } from '@/api/asm-state'

/**
 * Whether the bridge is already in safe harbor, read live from the node.
 *
 * `enabled` is what keeps the read off screens that have no lever answering it — a hook cannot
 * be called conditionally, so the condition lives here.
 *
 * A failed read degrades to `false` — no error surface. The only thing this state drives is a
 * note telling the council what the chain already says; a node that cannot answer must never
 * stand between them and the emergency lever, and an error banner on the dashboard would read
 * as a problem with the proposal list it sits above.
 */
export function useSafeHarborActivated(enabled = true): boolean {
	const [activated, setActivated] = useState(false)

	useEffect(() => {
		if (!enabled) return
		let cancelled = false
		void getSafeHarborStatus().then((result) => {
			if (cancelled) return
			setActivated(result.ok && result.data.activated)
		})
		return () => {
			cancelled = true
		}
	}, [enabled])

	return activated
}

/**
 * The whole safe harbor — activation *and* destination — for the one surface that needs both.
 *
 * Separate from `useSafeHarborActivated` because they degrade differently, and deliberately so.
 * The flag drives a note and must never block anything, so a failed read reads as `false`. The
 * destination is load-bearing: without it the create form can neither show what it is replacing nor
 * refuse a rotation to the address already installed, so a failed read has to be visible and the
 * caller has to stop. `safeHarbor === null` once `isLoading` is false is that signal.
 */
export function useSafeHarbor(enabled = true): { safeHarbor: SafeHarborStatus | null; isLoading: boolean } {
	const [safeHarbor, setSafeHarbor] = useState<SafeHarborStatus | null>(null)
	const [isLoading, setIsLoading] = useState(enabled)

	useEffect(() => {
		if (!enabled) {
			setIsLoading(false)
			return
		}
		let cancelled = false
		setIsLoading(true)
		void getSafeHarborStatus().then((result) => {
			if (cancelled) return
			setSafeHarbor(result.ok ? result.data : null)
			setIsLoading(false)
		})
		return () => {
			cancelled = true
		}
	}, [enabled])

	return { safeHarbor, isLoading }
}
