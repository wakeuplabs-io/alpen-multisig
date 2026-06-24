import type { WalletPanelSection } from '@/domain/admin-wallet/hooks/use-wallet-panel-state'
import type { AdminWalletError } from '@/domain/admin-wallet/model/types'
import type { AddressWithBalanceView, UnconfirmedTxView } from '@/domain/admin-wallet/model/view-models'
import type { SyncStatusDto } from '@/domain/admin-wallet/model/types'
import { networkFromPath } from '@/domain/admin-wallet/model/network-from-path'
import { DisabledWalletCard } from './disabled-wallet-card'
import { AdminIdRow } from './admin-id-row'
import { WalletBalance } from './wallet-balance'
import { ReceiveAddressRow } from './receive-address-row'
import { AddressesWithBalanceList } from './addresses-with-balance-list'
import { UnconfirmedTxsList } from './unconfirmed-txs-list'
import { SendForm } from './send-form'
import { SyncChip } from './sync-chip'

export type WalletPanelContentProps = {
	disabledError: AdminWalletError | null
	/** Canonical BIP-84 Admin ID (auth address) shown at the top of the panel (PRD §4.1). */
	adminId: string | undefined
	confirmedBalanceSats: number
	unconfirmedBalanceSats: number
	isBalanceLoading: boolean
	receiveAddress: string | null
	/** External index of the current receive address (verify-on-device path). */
	receiveIndex: number | null
	/** Device-accurate receive address (device-HRP) for the verify-on-device comparison. */
	receiveVerifyAddress: string | null
	/** Connected HW device (verify-on-device affordances), or null for software / no signer. */
	hwDeviceType: 'trezor' | 'ledger' | null
	/** Connect-returned Admin ID derivation path (authoritative verify-on-device path). */
	adminIdDerivationPath: string | undefined
	/** Connect-returned BIP-86 Admin Wallet account path (receive verify-on-device path). */
	adminWalletAccountPath: string | null
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
	onOpenSend(): void
	onCloseSend(): void
	syncStatus: SyncStatusDto | null
	isSyncRefreshing: boolean
	syncError: AdminWalletError | null
	onRefreshSync(): void
}

export function WalletPanelContent({
	disabledError,
	adminId,
	confirmedBalanceSats,
	unconfirmedBalanceSats,
	isBalanceLoading,
	receiveAddress,
	receiveIndex,
	receiveVerifyAddress,
	hwDeviceType,
	adminIdDerivationPath,
	adminWalletAccountPath,
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
	onOpenSend,
	onCloseSend,
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

	// Send sub-view (Phase 6, PRD §4.3.5): replaces the accordion stack while open.
	if (expandedSection === 'send') {
		return (
			<div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-5 py-5">
				<div className="mb-3 flex items-center gap-2">
					<button
						type="button"
						onClick={onCloseSend}
						aria-label="Back to wallet"
						className="flex h-6 w-6 items-center justify-center rounded-md border border-[#e5e7eb] bg-white text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
					>
						←
					</button>
					<h3 className="m-0 text-body-sm font-semibold text-[#111827]">Send BTC</h3>
				</div>
				<SendForm
					isWatchOnly={isWatchOnly}
					deviceType={hwDeviceType}
					onBack={onCloseSend}
					onAfterSend={onRefreshSync}
				/>
			</div>
		)
	}

	return (
		<div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-5 py-5">
			<div className="mb-4">
				<AdminIdRow
					adminId={adminId}
					verify={
						hwDeviceType && adminIdDerivationPath
							? {
									deviceType: hwDeviceType,
									network: networkFromPath(adminIdDerivationPath),
									derivationPath: adminIdDerivationPath,
								}
							: undefined
					}
				/>
			</div>

			<WalletBalance
				confirmedSats={confirmedBalanceSats}
				unconfirmedSats={unconfirmedBalanceSats}
				isLoading={isBalanceLoading}
			/>

			<div className="mt-4">
				<button
					type="button"
					onClick={onOpenSend}
					disabled={isWatchOnly}
					title={isWatchOnly ? 'Hardware wallet required to sign' : undefined}
					data-testid="e2e-wallet-send-open"
					className={`w-full rounded-lg px-3 py-2 text-body-sm font-medium transition ${
						isWatchOnly
							? 'cursor-not-allowed bg-[#f3f4f6] text-[#9ca3af]'
							: 'bg-[#111827] text-white hover:bg-[#1f2937]'
					}`}
				>
					Send
				</button>
				{isWatchOnly && <p className="m-0 mt-1 text-mono-sm text-[#9ca3af]">Hardware wallet required to sign</p>}
			</div>

			<div className="mt-5">
				<ReceiveAddressRow
					address={receiveAddress ?? ''}
					isLoading={isAddressesLoading}
					verify={
						hwDeviceType && receiveIndex !== null && adminWalletAccountPath
							? {
									deviceType: hwDeviceType,
									accountPath: adminWalletAccountPath,
									index: receiveIndex,
									verifyAddress: receiveVerifyAddress ?? undefined,
								}
							: undefined
					}
				/>
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
