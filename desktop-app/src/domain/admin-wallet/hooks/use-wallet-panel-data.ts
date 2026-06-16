import { useCallback, useEffect } from 'react'
import type { AdminWalletError } from '@/api/admin-wallet'
import { useWalletPanelState } from './use-wallet-panel-state'
import { useAdminWalletBalance } from './use-admin-wallet-balance'
import { useAdminWalletReceiveAddress } from './use-admin-wallet-receive-address'
import { useAdminWalletSync } from './use-admin-wallet-sync'
import { useAddressesWithBalance } from './use-addresses-with-balance'
import type { AddressWithBalanceView } from './use-addresses-with-balance'
import type { WalletPanelSection } from './use-wallet-panel-state'
import { useAdminWalletCapability } from './use-admin-wallet-capability'
import { useUnconfirmedTxs } from './use-unconfirmed-txs'
import type { UnconfirmedTxView } from '../model/compose-unconfirmed-tx-rows'

export type WalletPanelData = {
	isOpen: boolean
	open: () => void
	close: () => void
	toggle: () => void
	/** True when the session can view but not sign (HW connected without signing capability). */
	isWatchOnly: boolean
	confirmedBalanceSats: number
	unconfirmedBalanceSats: number
	isBalanceLoading: boolean
	receiveAddress: string | null
	isAddressesLoading: boolean
	addressRows: AddressWithBalanceView[] | null
	addressRowsLoading: boolean
	addressRowsError: ReturnType<typeof useAddressesWithBalance>['error']
	unconfirmedTxRows: UnconfirmedTxView[] | null
	unconfirmedTxsLoading: boolean
	unconfirmedTxsError: AdminWalletError | null
	expandedSection: WalletPanelSection | null
	syncStatus: ReturnType<typeof useAdminWalletSync>['syncStatus']
	isSyncRefreshing: boolean
	syncError: ReturnType<typeof useAdminWalletSync>['error'] | null
	onToggleAddresses: () => void
	onToggleTransactions: () => void
	onOpenSend: () => void
	onCloseSend: () => void
	onRefreshSync: () => Promise<void>
	disabledError: AdminWalletError | null
}

export function useWalletPanelData(showDisabledError: boolean = true): WalletPanelData {
	const { isOpen, expandedSection, open, close, setExpandedSection } = useWalletPanelState()
	const balanceHook = useAdminWalletBalance()
	const receiveAddressHook = useAdminWalletReceiveAddress()
	const syncHook = useAdminWalletSync()
	const addressesWithBalanceHook = useAddressesWithBalance()
	const unconfirmedTxsHook = useUnconfirmedTxs()
	const { canSign } = useAdminWalletCapability()

	// All refresh callbacks below are referentially stable (each is a `useCallback(…, [])`
	// or depends only on other stable callbacks), so the effect/useCallback deps stay
	// stable across renders. Do NOT add `syncHook.isLoading` (or other per-render state)
	// to these deps — that would re-fire the open effect on every load toggle and loop.
	const { refresh: refreshBalance } = balanceHook
	const { refresh: refreshReceiveAddress } = receiveAddressHook
	const { refresh: refreshAddresses } = addressesWithBalanceHook
	const { refresh: refreshUnconfirmedTxs } = unconfirmedTxsHook
	const { triggerSync } = syncHook

	// Sync the BDK wallet against the chain, then re-read the derived views. Without the
	// sync, the reads return a stale/zero balance ("Never synced"). Shared by the on-open
	// auto-sync and the manual Refresh button.
	const syncAndRefresh = useCallback(async () => {
		await triggerSync()
		refreshBalance()
		refreshReceiveAddress()
		refreshAddresses()
		refreshUnconfirmedTxs()
	}, [triggerSync, refreshBalance, refreshReceiveAddress, refreshAddresses, refreshUnconfirmedTxs])

	// Initial refresh: when the panel opens, sync + re-read once so the signer sees a
	// fresh, chain-synced balance without having to press Refresh manually.
	useEffect(() => {
		if (!isOpen) return
		void syncAndRefresh()
	}, [isOpen, syncAndRefresh])

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
		toggle: () => (isOpen ? close() : open()),
		isWatchOnly: !canSign,
		confirmedBalanceSats: balanceHook.data?.confirmedSats ?? 0,
		unconfirmedBalanceSats: balanceHook.data?.unconfirmedSats ?? 0,
		isBalanceLoading: balanceHook.isLoading,
		receiveAddress: receiveAddressHook.address,
		isAddressesLoading: receiveAddressHook.isLoading,
		addressRows: addressesWithBalanceHook.data,
		addressRowsLoading: addressesWithBalanceHook.isLoading,
		addressRowsError: addressesWithBalanceHook.error,
		unconfirmedTxRows: unconfirmedTxsHook.data,
		unconfirmedTxsLoading: unconfirmedTxsHook.isLoading,
		unconfirmedTxsError: unconfirmedTxsHook.error,
		expandedSection,
		syncStatus: syncHook.syncStatus,
		isSyncRefreshing: syncHook.isLoading,
		syncError: syncHook.error,
		onToggleAddresses: () => setExpandedSection(expandedSection === 'addresses' ? null : 'addresses'),
		onToggleTransactions: () => setExpandedSection(expandedSection === 'transactions' ? null : 'transactions'),
		onOpenSend: () => setExpandedSection('send'),
		onCloseSend: () => setExpandedSection(null),
		onRefreshSync: syncAndRefresh,
		disabledError: showDisabledError ? walletDisabledError : null,
	}
}
