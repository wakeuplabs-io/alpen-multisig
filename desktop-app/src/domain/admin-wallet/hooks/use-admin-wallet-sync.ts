import { useCallback, useEffect, useState } from 'react'
import { triggerAdminWalletSync, getAdminWalletSyncStatus } from '@/api/admin-wallet'
import type { SyncStatusDto, AdminWalletError } from '@/api/admin-wallet'
import { parseAdminWalletError } from './parse-admin-wallet-error'

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

	const refresh = useCallback(() => setTick((t) => t + 1), [])

	const triggerSync = useCallback(async () => {
		setIsLoading(true)
		const result = await triggerAdminWalletSync()
		if (!result.ok) {
			setError(parseAdminWalletError(result.error))
		}
		setIsLoading(false)
		refresh()
	}, [refresh])

	return { syncStatus, isLoading, error, refresh, triggerSync }
}
