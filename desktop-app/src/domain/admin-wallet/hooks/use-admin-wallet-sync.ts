import { useEffect, useState } from 'react'
import { triggerAdminWalletSync, getAdminWalletSyncStatus } from '@/api/admin-wallet'
import type { SyncStatusDto, AdminWalletError } from '@/api/admin-wallet'

type UseAdminWalletSyncReturn = {
	syncStatus: SyncStatusDto | null
	isLoading: boolean
	error: AdminWalletError | null
	refresh: () => void
	triggerSync: () => Promise<void>
}

function parseAdminWalletError(raw: string): AdminWalletError {
	try {
		const parsed = JSON.parse(raw) as AdminWalletError
		return parsed
	} catch {
		return { type: 'RpcUnreachable', message: raw }
	}
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

	function refresh() {
		setTick((t) => t + 1)
	}

	async function triggerSync() {
		setIsLoading(true)
		const result = await triggerAdminWalletSync()
		if (!result.ok) {
			setError(parseAdminWalletError(result.error))
		}
		setIsLoading(false)
		refresh()
	}

	return { syncStatus, isLoading, error, refresh, triggerSync }
}
