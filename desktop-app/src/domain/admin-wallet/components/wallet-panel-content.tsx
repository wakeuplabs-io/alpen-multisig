import type { WalletPanelSection } from '@/domain/admin-wallet/hooks/use-wallet-panel-state'
import type { AdminWalletError } from '@/domain/admin-wallet/model/types'
import type { AddressWithBalanceView } from '@/domain/admin-wallet/model/view-models'
import type { SyncStatusDto } from '@/domain/admin-wallet/model/types'
import { DisabledWalletCard } from './disabled-wallet-card'
import { WalletBalance } from './wallet-balance'
import { ReceiveAddressRow } from './receive-address-row'
import { AddressesWithBalanceList } from './addresses-with-balance-list'
import { ReceiveSection } from './receive-section'
import { TxHistoryList } from './tx-history-list'
import { SendPlaceholder } from './send-placeholder'
import { SyncChip } from './sync-chip'

export type WalletPanelContentProps = {
	disabledError: AdminWalletError | null
	balanceSats: number
	isBalanceLoading: boolean
	receiveAddress: string | null
	isAddressesLoading: boolean
	addressRows: AddressWithBalanceView[] | null
	addressRowsLoading: boolean
	addressRowsError: AdminWalletError | null
	expandedSection: WalletPanelSection | null
	onToggleAddresses(): void
	syncStatus: SyncStatusDto | null
	isSyncRefreshing: boolean
	syncError: AdminWalletError | null
	onRefreshSync(): void
}

export function WalletPanelContent({
	disabledError,
	balanceSats,
	isBalanceLoading,
	receiveAddress,
	isAddressesLoading,
	addressRows,
	addressRowsLoading,
	addressRowsError,
	expandedSection,
	onToggleAddresses,
	syncStatus,
	isSyncRefreshing,
	syncError,
	onRefreshSync,
}: WalletPanelContentProps) {
	if (disabledError !== null && (disabledError.type === 'Disabled' || disabledError.type === 'RegtestGuardViolation')) {
		return (
			<div className="p-4">
				<DisabledWalletCard error={disabledError} />
			</div>
		)
	}

	return (
		<div className="flex flex-col gap-0 overflow-y-auto">
			<WalletBalance balanceSats={balanceSats} isLoading={isBalanceLoading} />
			<div className="px-5 py-3">
				<ReceiveAddressRow address={receiveAddress ?? ''} isLoading={isAddressesLoading} />
			</div>
			<AddressesWithBalanceList
				rows={addressRows}
				isLoading={addressRowsLoading}
				error={addressRowsError}
				isExpanded={expandedSection === 'addresses'}
				onToggle={onToggleAddresses}
			/>
			<div className="px-5 py-3">
				<ReceiveSection address={receiveAddress} isLoading={isAddressesLoading} />
			</div>
			<div className="px-5 py-3">
				<TxHistoryList />
			</div>
			<div className="px-5 py-3">
				<SendPlaceholder />
			</div>
			<div className="px-5 py-3">
				<SyncChip syncStatus={syncStatus} isRefreshing={isSyncRefreshing} error={syncError} onRefresh={onRefreshSync} />
			</div>
		</div>
	)
}
