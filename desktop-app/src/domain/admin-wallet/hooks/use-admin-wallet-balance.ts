import { useCallback, useEffect, useState } from 'react'
import { getAdminWalletBalance } from '@/api/admin-wallet'
import type { BalanceDto, AdminWalletError } from '@/api/admin-wallet'
import { parseAdminWalletError } from './parse-admin-wallet-error'

type UseAdminWalletBalanceReturn = {
	data: BalanceDto | null
	isLoading: boolean
	error: AdminWalletError | null
	refresh: () => void
}

export function useAdminWalletBalance(): UseAdminWalletBalanceReturn {
	const [data, setData] = useState<BalanceDto | null>(null)
	const [isLoading, setIsLoading] = useState(false)
	const [error, setError] = useState<AdminWalletError | null>(null)
	const [tick, setTick] = useState(0)

	useEffect(() => {
		setIsLoading(true)
		getAdminWalletBalance()
			.then((result) => {
				if (result.ok) {
					setData(result.data)
					setError(null)
				} else {
					setError(parseAdminWalletError(result.error))
				}
			})
			.finally(() => setIsLoading(false))
	}, [tick])

	const refresh = useCallback(() => setTick((t) => t + 1), [])

	return { data, isLoading, error, refresh }
}
