import { useCallback, useEffect, useState } from 'react'
import { listAdminWalletAddresses } from '@/api/admin-wallet'
import type { AddressDto, AdminWalletError, KeychainDto } from '@/api/admin-wallet'
import { parseAdminWalletError } from './parse-admin-wallet-error'

type UseAdminWalletAddressesReturn = {
	data: AddressDto[] | null
	isLoading: boolean
	error: AdminWalletError | null
	refresh: () => void
}

export function useAdminWalletAddresses(
	keychain: KeychainDto = 'External',
	pageIndex = 0,
	pageSize = 20,
): UseAdminWalletAddressesReturn {
	const [data, setData] = useState<AddressDto[] | null>(null)
	const [isLoading, setIsLoading] = useState(false)
	const [error, setError] = useState<AdminWalletError | null>(null)
	const [tick, setTick] = useState(0)

	const clampedPageSize = Math.min(pageSize, 20)

	useEffect(() => {
		setIsLoading(true)
		listAdminWalletAddresses(keychain, pageIndex, clampedPageSize)
			.then((result) => {
				if (result.ok) {
					setData(result.data)
					setError(null)
				} else {
					setError(parseAdminWalletError(result.error))
				}
			})
			.finally(() => setIsLoading(false))
	}, [tick, keychain, pageIndex, clampedPageSize])

	const refresh = useCallback(() => setTick((t) => t + 1), [])

	return { data, isLoading, error, refresh }
}
