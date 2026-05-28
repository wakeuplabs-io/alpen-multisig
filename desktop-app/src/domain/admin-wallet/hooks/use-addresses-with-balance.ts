import { useCallback } from 'react'
import { useAdminWalletAddresses } from './use-admin-wallet-addresses'
import { useAdminWalletUtxos } from './use-admin-wallet-utxos'
import { composeAddressesWithBalance, type AddressWithBalanceView } from '../model/compose-addresses-with-balance'
import type { AdminWalletError, KeychainDto } from '@/api/admin-wallet'

export type { AddressWithBalanceView }

type UseAddressesWithBalanceReturn = {
	data: AddressWithBalanceView[] | null
	isLoading: boolean
	error: AdminWalletError | null
	refresh(): void
}

export function useAddressesWithBalance(opts?: {
	keychain?: KeychainDto
	pageIndex?: number
	pageSize?: number
}): UseAddressesWithBalanceReturn {
	const addressesHook = useAdminWalletAddresses(
		opts?.keychain ?? 'External',
		opts?.pageIndex ?? 0,
		opts?.pageSize ?? 20,
	)
	const utxosHook = useAdminWalletUtxos()

	const isLoading = addressesHook.isLoading || utxosHook.isLoading
	const error = addressesHook.error ?? utxosHook.error

	const data =
		addressesHook.data !== null && utxosHook.data !== null
			? composeAddressesWithBalance(addressesHook.data, utxosHook.data)
			: null

	const refresh = useCallback(() => {
		addressesHook.refresh()
		utxosHook.refresh()
	}, [addressesHook, utxosHook])

	return { data, isLoading, error, refresh }
}
