import type { AdminWalletError } from '@/api/admin-wallet'
import { useWalletPanelState } from './use-wallet-panel-state'
import { useAdminWalletBalance } from './use-admin-wallet-balance'
import { useAdminWalletReceiveAddress } from './use-admin-wallet-receive-address'
import { useAdminWalletSync } from './use-admin-wallet-sync'
import { useAddressesWithBalance } from './use-addresses-with-balance'
import type { AddressWithBalanceView } from './use-addresses-with-balance'
import type { WalletPanelSection } from './use-wallet-panel-state'

export type WalletPanelData = {
	isOpen: boolean
	open: () => void
	close: () => void
	confirmedBalanceSats: number
	unconfirmedBalanceSats: number
	isBalanceLoading: boolean
	receiveAddress: string | null
	isAddressesLoading: boolean
	addressRows: AddressWithBalanceView[] | null
	addressRowsLoading: boolean
	addressRowsError: ReturnType<typeof useAddressesWithBalance>['error']
	expandedSection: WalletPanelSection | null
	syncStatus: ReturnType<typeof useAdminWalletSync>['syncStatus']
	isSyncRefreshing: boolean
	syncError: ReturnType<typeof useAdminWalletSync>['error'] | null
	onToggleAddresses: () => void
	onRefreshSync: () => Promise<void>
	disabledError: AdminWalletError | null
}

export function useWalletPanelData(showDisabledError: boolean = true): WalletPanelData {
	const { isOpen, expandedSection, open, close, setExpandedSection } = useWalletPanelState()
	const balanceHook = useAdminWalletBalance()
	const receiveAddressHook = useAdminWalletReceiveAddress()
	const syncHook = useAdminWalletSync()
	const addressesWithBalanceHook = useAddressesWithBalance()

	const walletDisabledError =
		balanceHook.error?.type === 'Disabled' || balanceHook.error?.type === 'RegtestGuardViolation'
			? balanceHook.error
			: receiveAddressHook.error?.type === 'Disabled' || receiveAddressHook.error?.type === 'RegtestGuardViolation'
				? receiveAddressHook.error
				: null

	return {
		isOpen,
		open,
		close,
		confirmedBalanceSats: balanceHook.data?.confirmedSats ?? 0,
		unconfirmedBalanceSats: balanceHook.data?.unconfirmedSats ?? 0,
		isBalanceLoading: balanceHook.isLoading,
		receiveAddress: receiveAddressHook.address,
		isAddressesLoading: receiveAddressHook.isLoading,
		addressRows: addressesWithBalanceHook.data,
		addressRowsLoading: addressesWithBalanceHook.isLoading,
		addressRowsError: addressesWithBalanceHook.error,
		expandedSection,
		syncStatus: syncHook.syncStatus,
		isSyncRefreshing: syncHook.isLoading,
		syncError: syncHook.error,
		onToggleAddresses: () => setExpandedSection(expandedSection === 'addresses' ? null : 'addresses'),
		onRefreshSync: async () => {
			await syncHook.triggerSync()
			balanceHook.refresh()
			receiveAddressHook.refresh()
			addressesWithBalanceHook.refresh()
		},
		disabledError: showDisabledError ? walletDisabledError : null,
	}
}
