import { useEffect, useState } from 'react'
import { listAdminWalletAddresses } from '@/api/admin-wallet'
import type { AddressDto, AdminWalletError } from '@/api/admin-wallet'

type UseAdminWalletAddressesReturn = {
	data: AddressDto[] | null
	isLoading: boolean
	error: AdminWalletError | null
	refresh: () => void
}

function parseAdminWalletError(raw: string): AdminWalletError {
	try {
		const parsed = JSON.parse(raw) as AdminWalletError
		return parsed
	} catch {
		return { type: 'RpcUnreachable', message: raw }
	}
}

export function useAdminWalletAddresses(pageSize = 20): UseAdminWalletAddressesReturn {
	const [data, setData] = useState<AddressDto[] | null>(null)
	const [isLoading, setIsLoading] = useState(false)
	const [error, setError] = useState<AdminWalletError | null>(null)
	const [tick, setTick] = useState(0)

	const clampedPageSize = Math.min(pageSize, 20)

	useEffect(() => {
		setIsLoading(true)
		listAdminWalletAddresses(clampedPageSize)
			.then((result) => {
				if (result.ok) {
					setData(result.data)
					setError(null)
				} else {
					setError(parseAdminWalletError(result.error))
				}
			})
			.finally(() => setIsLoading(false))
	}, [tick, clampedPageSize])

	function refresh() {
		setTick((t) => t + 1)
	}

	return { data, isLoading, error, refresh }
}
