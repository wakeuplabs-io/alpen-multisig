import { useEffect } from 'react'
import { BLOCK_HEIGHT_POLL_INTERVAL_MS } from '@/hooks/use-block-height'

/**
 * While a proposal has reached its activation block, read it again on the tip's own interval (#602).
 *
 * Per block is not enough: the tip comes from bitcoind and enactment is decided against the ASM's
 * anchor, so the first read can land while the ASM is still a block behind. `refresh` must be
 * stable, or this restarts the interval on every render.
 */
export function useActivationRefresh(isActivationReached: boolean, refresh: () => void): void {
	useEffect(() => {
		if (!isActivationReached) return
		refresh()
		const id = setInterval(refresh, BLOCK_HEIGHT_POLL_INTERVAL_MS)
		return () => clearInterval(id)
	}, [isActivationReached, refresh])
}
