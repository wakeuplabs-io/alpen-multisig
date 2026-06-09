import { useCallback } from 'react'
import { useAdminWalletAddresses } from './use-admin-wallet-addresses'
import { useAdminWalletUtxos } from './use-admin-wallet-utxos'
import { composeAddressesWithBalance, type AddressWithBalanceView } from '../model/compose-addresses-with-balance'
import type { AdminWalletError } from '@/api/admin-wallet'

export type { AddressWithBalanceView }

type UseAddressesWithBalanceReturn = {
	data: AddressWithBalanceView[] | null
	isLoading: boolean
	error: AdminWalletError | null
	refresh(): void
}

export function useAddressesWithBalance(opts?: {
	pageIndex?: number
	pageSize?: number
}): UseAddressesWithBalanceReturn {
	// Both keychains: change (internal) addresses hold the remaining funds after a spend and
	// must show up in "Addresses with balance" alongside receive (external) addresses.
	const externalHook = useAdminWalletAddresses('External', opts?.pageIndex ?? 0, opts?.pageSize ?? 20)
	const internalHook = useAdminWalletAddresses('Internal', opts?.pageIndex ?? 0, opts?.pageSize ?? 20)
	const utxosHook = useAdminWalletUtxos()

	const isLoading = externalHook.isLoading || internalHook.isLoading || utxosHook.isLoading
	const error = externalHook.error ?? internalHook.error ?? utxosHook.error

	const data =
		externalHook.data !== null && internalHook.data !== null && utxosHook.data !== null
			? composeAddressesWithBalance({ external: externalHook.data, internal: internalHook.data }, utxosHook.data)
			: null

	// Depend on the stable inner `refresh` callbacks, not the hook objects (which are
	// new on every render). Depending on the objects would make this `refresh` change
	// every render, breaking referential stability for any effect that depends on it.
	const { refresh: refreshExternal } = externalHook
	const { refresh: refreshInternal } = internalHook
	const { refresh: refreshUtxos } = utxosHook
	const refresh = useCallback(() => {
		refreshExternal()
		refreshInternal()
		refreshUtxos()
	}, [refreshExternal, refreshInternal, refreshUtxos])

	return { data, isLoading, error, refresh }
}
