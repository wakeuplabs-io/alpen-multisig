import { useEffect, useState } from 'react'
import { hasStoredBundle } from '@/api/proposals'

/**
 * How often the stored bundle is read again. The settle loop can drop a bundle without the row
 * changing (#605), so `refreshKey` alone would keep a stale answer on screen.
 */
const STORED_BUNDLE_REFRESH_MS = 10_000

/**
 * Whether this desktop still keeps a commit + reveal bundle for the proposal (#605), read again
 * whenever `refreshKey` changes and every few seconds. `null` while unknown, or when the desktop
 * could not say.
 */
export function useStoredBundle(actionId: string | undefined, refreshKey: unknown): boolean | null {
	const [holdsBundle, setHoldsBundle] = useState<boolean | null>(null)

	useEffect(() => {
		if (actionId === undefined) return
		let active = true
		const read = () => {
			void hasStoredBundle(actionId).then((res) => {
				if (active) setHoldsBundle(res.ok ? res.data : null)
			})
		}
		read()
		const interval = setInterval(read, STORED_BUNDLE_REFRESH_MS)
		return () => {
			active = false
			clearInterval(interval)
		}
	}, [actionId, refreshKey])

	return holdsBundle
}
