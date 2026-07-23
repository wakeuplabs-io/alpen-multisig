import { useEffect, useState } from 'react'
import { getMultisigConfig } from '@/api/asm-state'

/**
 * Reads the authority's on-chain signing threshold so the review screen can tell an
 * actual threshold change from one the proposal merely restates (#423).
 *
 * Returns `null` while loading or when the config cannot be read — callers must treat
 * that as "unknown" and keep showing the proposed value rather than hiding it.
 */
export function useCurrentThreshold(authority: string, enabled: boolean): number | null {
	const [currentThreshold, setCurrentThreshold] = useState<number | null>(null)

	useEffect(() => {
		if (!enabled) {
			setCurrentThreshold(null)
			return
		}

		let cancelled = false
		void getMultisigConfig(authority).then((result) => {
			if (cancelled) {
				return
			}
			setCurrentThreshold(result.ok ? result.data.threshold : null)
		})

		return () => {
			cancelled = true
		}
	}, [authority, enabled])

	return currentThreshold
}
