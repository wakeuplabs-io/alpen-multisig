import { useCallback, useEffect, useState } from 'react'
import { getAdminWalletInfo } from '@/api/admin-wallet'

type AdminWalletInfoView = {
	address: string
	balanceSats: number
}

// The wallet can be synced from anywhere (wallet panel Refresh, background sync, another
// screen's trigger) — poll the cached wallet info so the funding card follows along instead of
// freezing on its mount-time snapshot. The read is local (no indexer round trip), so a short
// interval is cheap.
const INFO_POLL_INTERVAL_MS = 5000

/** Reference-stable update: keeps the previous object when nothing changed, so the 5s poll does
 * not re-render consumers on every tick. */
export function nextAdminWalletInfoState(
	prev: AdminWalletInfoView | null | undefined,
	next: AdminWalletInfoView,
): AdminWalletInfoView {
	return prev != null && prev.address === next.address && prev.balanceSats === next.balanceSats ? prev : next
}

type UseAdminWalletInfoReturn = {
	/** null = loading, undefined = unavailable/error, AdminWalletInfoView = loaded */
	adminWalletInfo: AdminWalletInfoView | null | undefined
	/** Re-reads the wallet info immediately — call after a sync this screen triggered itself. */
	refresh: () => void
}

export function useAdminWalletInfo(sessionReady: boolean): UseAdminWalletInfoReturn {
	const [adminWalletInfo, setAdminWalletInfo] = useState<AdminWalletInfoView | null | undefined>(null)
	const [tick, setTick] = useState(0)

	useEffect(() => {
		if (!sessionReady) return
		let cancelled = false

		const fetchInfo = () => {
			getAdminWalletInfo().then((result) => {
				if (cancelled) return
				if (result.ok) {
					const next = { address: result.data.address, balanceSats: result.data.balance_sats }
					setAdminWalletInfo((prev) => nextAdminWalletInfoState(prev, next))
				} else {
					console.warn('[admin-wallet-info] getAdminWalletInfo failed:', result.error)
					setAdminWalletInfo(undefined)
				}
			})
		}

		fetchInfo()
		const interval = setInterval(fetchInfo, INFO_POLL_INTERVAL_MS)
		return () => {
			cancelled = true
			clearInterval(interval)
		}
	}, [sessionReady, tick])

	const refresh = useCallback(() => setTick((t) => t + 1), [])

	return { adminWalletInfo, refresh }
}
