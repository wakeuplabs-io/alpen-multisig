import type { WalletPanelSection } from '@/domain/admin-wallet/hooks/use-wallet-panel-state'
import type { AdminWalletError } from '@/domain/admin-wallet/model/types'
import type { AddressWithBalanceView, UnconfirmedTxView } from '@/domain/admin-wallet/model/view-models'
import type { SyncStatusDto } from '@/domain/admin-wallet/model/types'
import { DisabledWalletCard } from './disabled-wallet-card'
import { WalletBalance } from './wallet-balance'
import { ReceiveAddressRow } from './receive-address-row'
import { AddressesWithBalanceList } from './addresses-with-balance-list'
import { UnconfirmedTxsList } from './unconfirmed-txs-list'
import { SyncChip } from './sync-chip'

export type WalletPanelContentProps = {
	disabledError: AdminWalletError | null
	confirmedBalanceSats: number
	unconfirmedBalanceSats: number
	isBalanceLoading: boolean
	receiveAddress: string | null
	isAddressesLoading: boolean
	addressRows: AddressWithBalanceView[] | null
	addressRowsLoading: boolean
	addressRowsError: AdminWalletError | null
	unconfirmedTxRows: UnconfirmedTxView[] | null
	unconfirmedTxsLoading: boolean
	unconfirmedTxsError: AdminWalletError | null
	isWatchOnly: boolean
	expandedSection: WalletPanelSection | null
	onToggleAddresses(): void
	onToggleTransactions(): void
	syncStatus: SyncStatusDto | null
	isSyncRefreshing: boolean
	syncError: AdminWalletError | null
	onRefreshSync(): void
}

export function WalletPanelContent({
	disabledError,
	confirmedBalanceSats,
	unconfirmedBalanceSats,
	isBalanceLoading,
	receiveAddress,
	isAddressesLoading,
	addressRows,
	addressRowsLoading,
	addressRowsError,
	unconfirmedTxRows,
	unconfirmedTxsLoading,
	unconfirmedTxsError,
	isWatchOnly,
	expandedSection,
	onToggleAddresses,
	onToggleTransactions,
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
		<div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-5 py-5">
			<WalletBalance
				confirmedSats={confirmedBalanceSats}
				unconfirmedSats={unconfirmedBalanceSats}
				isLoading={isBalanceLoading}
			/>

			<div className="mt-5">
				<ReceiveAddressRow address={receiveAddress ?? ''} isLoading={isAddressesLoading} />
			</div>

			<div className="mt-5 border-t border-[#f3f4f6] pt-4">
				<AddressesWithBalanceList
					rows={addressRows}
					isLoading={addressRowsLoading}
					error={addressRowsError}
					isExpanded={expandedSection === 'addresses'}
					onToggle={onToggleAddresses}
				/>
			</div>

			<div className="mt-2 border-t border-[#f3f4f6] pt-2">
				<UnconfirmedTxsList
					rows={unconfirmedTxRows}
					isLoading={unconfirmedTxsLoading}
					error={unconfirmedTxsError}
					isExpanded={expandedSection === 'transactions'}
					onToggle={onToggleTransactions}
					isWatchOnly={isWatchOnly}
					onAfterBump={onRefreshSync}
				/>
			</div>

			<div className="mt-auto border-t border-[#f3f4f6] pt-4">
				<SyncChip syncStatus={syncStatus} isRefreshing={isSyncRefreshing} error={syncError} onRefresh={onRefreshSync} />
			</div>
		</div>
	)
}
