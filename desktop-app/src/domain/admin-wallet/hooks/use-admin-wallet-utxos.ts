import { useCallback, useEffect, useState } from 'react'
import { listAdminWalletUtxos } from '@/api/admin-wallet'
import type { UtxoDto, AdminWalletError } from '@/api/admin-wallet'
import { parseAdminWalletError } from './parse-admin-wallet-error'

type UseAdminWalletUtxosReturn = {
	data: UtxoDto[] | null
	isLoading: boolean
	error: AdminWalletError | null
	refresh: () => void
}

export function useAdminWalletUtxos(): UseAdminWalletUtxosReturn {
	const [data, setData] = useState<UtxoDto[] | null>(null)
	const [isLoading, setIsLoading] = useState(false)
	const [error, setError] = useState<AdminWalletError | null>(null)
	const [tick, setTick] = useState(0)

	useEffect(() => {
		setIsLoading(true)
		listAdminWalletUtxos()
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
