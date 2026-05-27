import { useEffect, useState } from 'react'
import { listAdminWalletUtxos } from '@/api/admin-wallet'
import type { UtxoDto, AdminWalletError } from '@/api/admin-wallet'

type UseAdminWalletUtxosReturn = {
	data: UtxoDto[] | null
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

	function refresh() {
		setTick((t) => t + 1)
	}

	return { data, isLoading, error, refresh }
}
