import { useCallback, useEffect, useState } from 'react'
import { triggerAdminWalletSync, getAdminWalletSyncStatus } from '@/api/admin-wallet'
import type { SyncStatusDto, AdminWalletError } from '@/api/admin-wallet'
import { parseAdminWalletError } from './parse-admin-wallet-error'

// How often to poll sync status while a sync is in flight, so the >3s progress indicator
// updates live. `triggerSync()` awaits the whole sync, so without polling the UI would never
// observe intermediate progress.
const SYNC_POLL_INTERVAL_MS = 1000

type UseAdminWalletSyncReturn = {
	syncStatus: SyncStatusDto | null
	isLoading: boolean
	error: AdminWalletError | null
	refresh: () => void
	triggerSync: () => Promise<void>
}

export function useAdminWalletSync(): UseAdminWalletSyncReturn {
	const [syncStatus, setSyncStatus] = useState<SyncStatusDto | null>(null)
	const [isLoading, setIsLoading] = useState(false)
	const [error, setError] = useState<AdminWalletError | null>(null)
	const [tick, setTick] = useState(0)
	const [isSyncing, setIsSyncing] = useState(false)

	useEffect(() => {
		setIsLoading(true)
		getAdminWalletSyncStatus()
			.then((result) => {
				if (result.ok) {
					setSyncStatus(result.data)
					setError(null)
				} else {
					setError(parseAdminWalletError(result.error))
				}
			})
			.finally(() => setIsLoading(false))
	}, [tick])

	// While a sync is in flight, poll status so the >3s progress indicator updates live.
	// Keyed on `isSyncing` only — this stays internal to this hook and must NOT leak into the
	// panel's on-open effect deps (those must remain stable callbacks, or the panel loops).
	useEffect(() => {
		if (!isSyncing) return
		const interval = setInterval(() => {
			void getAdminWalletSyncStatus().then((result) => {
				if (result.ok) setSyncStatus(result.data)
			})
		}, SYNC_POLL_INTERVAL_MS)
		return () => clearInterval(interval)
	}, [isSyncing])

	const refresh = useCallback(() => setTick((t) => t + 1), [])

	const triggerSync = useCallback(async () => {
		setIsLoading(true)
		setIsSyncing(true)
		try {
			const result = await triggerAdminWalletSync()
			if (!result.ok) {
				setError(parseAdminWalletError(result.error))
			}
		} finally {
			setIsSyncing(false)
			setIsLoading(false)
			refresh()
		}
	}, [refresh])

	return { syncStatus, isLoading, error, refresh, triggerSync }
}
