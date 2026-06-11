import { useCallback, useEffect, useState } from 'react'
import { listAdminWalletUnconfirmedTxs } from '@/api/admin-wallet'
import type { AdminWalletError } from '@/api/admin-wallet'
import { composeUnconfirmedTxRows, type UnconfirmedTxView } from '../model/compose-unconfirmed-tx-rows'
import { parseAdminWalletError } from './parse-admin-wallet-error'

type UseUnconfirmedTxsReturn = {
	data: UnconfirmedTxView[] | null
	isLoading: boolean
	error: AdminWalletError | null
	refresh(): void
}

/** Unconfirmed transactions sent from the Admin Wallet (Phase 5, PRD §4.3.3), as display rows. */
export function useUnconfirmedTxs(): UseUnconfirmedTxsReturn {
	const [data, setData] = useState<UnconfirmedTxView[] | null>(null)
	const [isLoading, setIsLoading] = useState(false)
	const [error, setError] = useState<AdminWalletError | null>(null)
	const [tick, setTick] = useState(0)

	useEffect(() => {
		setIsLoading(true)
		listAdminWalletUnconfirmedTxs()
			.then((result) => {
				if (result.ok) {
					setData(composeUnconfirmedTxRows(result.data))
					setError(null)
				} else {
					setError(parseAdminWalletError(result.error))
				}
			})
			.finally(() => setIsLoading(false))
	}, [tick])

	// Stable identity — safe to use in the panel's on-open effect deps.
	const refresh = useCallback(() => setTick((t) => t + 1), [])

	return { data, isLoading, error, refresh }
}
