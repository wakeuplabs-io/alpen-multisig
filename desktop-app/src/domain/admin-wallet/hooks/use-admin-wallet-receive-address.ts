import { useCallback, useEffect, useState } from 'react'
import { nextAdminWalletReceiveAddress } from '@/api/admin-wallet'
import type { AddressDto, AdminWalletError } from '@/api/admin-wallet'
import { parseAdminWalletError } from './parse-admin-wallet-error'

type UseAdminWalletReceiveAddressReturn = {
	address: string | null
	index: number | null
	/** Device-accurate address (device-HRP) for verify-on-device; null for software sessions. */
	verifyAddress: string | null
	isLoading: boolean
	error: AdminWalletError | null
	refresh: () => void
}

// R1.3 (receive rotation): fetches the next unused external (receive) address from the
// backend. The backend is idempotent until the address is credited and observed during
// sync; calling refresh() after a sync surfaces the rotated address.
export function useAdminWalletReceiveAddress(): UseAdminWalletReceiveAddressReturn {
	const [data, setData] = useState<AddressDto | null>(null)
	const [isLoading, setIsLoading] = useState(false)
	const [error, setError] = useState<AdminWalletError | null>(null)
	const [tick, setTick] = useState(0)

	useEffect(() => {
		setIsLoading(true)
		nextAdminWalletReceiveAddress()
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

	return {
		address: data?.address ?? null,
		index: data?.index ?? null,
		verifyAddress: data?.verifyAddress ?? null,
		isLoading,
		error,
		refresh,
	}
}
