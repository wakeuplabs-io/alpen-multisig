import { SessionChip } from '@/components/session-chip'
import type { WalletPanelData } from '../hooks/use-wallet-panel-data'
import { WalletPanel, WALLET_PANEL_ID } from './wallet-panel'
import { WalletPanelHeader } from './wallet-panel-header'
import { WalletPanelContent } from './wallet-panel-content'

const SIGNER_LABEL_PREFIX = 10
const SIGNER_LABEL_SUFFIX = 8

function formatSignerLabel(addressSample: string | undefined): string {
	if (!addressSample) return 'Unknown'
	return `${addressSample.slice(0, SIGNER_LABEL_PREFIX)}…${addressSample.slice(-SIGNER_LABEL_SUFFIX)}`
}

export type WalletSessionControlProps = {
	panel: WalletPanelData
	sessionTimeLabel: string
	sessionWarning: boolean
	/** Truncated signer address shown in the chip and panel subtitle. */
	addressSample: string | undefined
}

/**
 * Header session chip wired to the slide-in wallet panel. Renders both the
 * trigger and the dialog (the dialog is `position: fixed`, so it escapes the
 * header flow). Place inside `ScreenShell`'s `headerContent` alongside the
 * authority badge and disconnect button.
 */
export function WalletSessionControl({
	panel,
	sessionTimeLabel,
	sessionWarning,
	addressSample,
}: WalletSessionControlProps) {
	const signerLabel = formatSignerLabel(addressSample)

	return (
		<>
			<SessionChip
				timeLabel={sessionTimeLabel}
				signerLabel={signerLabel}
				warning={sessionWarning}
				onActivate={panel.toggle}
				isActive={panel.isOpen}
				panelId={WALLET_PANEL_ID}
			/>
			<WalletPanel isOpen={panel.isOpen} onClose={panel.close} panelId={WALLET_PANEL_ID}>
				<WalletPanelHeader
					onClose={panel.close}
					subtitle={`Session · ${sessionTimeLabel}`}
					isWatchOnly={panel.isWatchOnly}
				/>
				<WalletPanelContent
					disabledError={panel.disabledError}
					adminId={addressSample}
					confirmedBalanceSats={panel.confirmedBalanceSats}
					unconfirmedBalanceSats={panel.unconfirmedBalanceSats}
					isBalanceLoading={panel.isBalanceLoading}
					receiveAddress={panel.receiveAddress}
					receiveIndex={panel.receiveIndex}
					hwDeviceType={panel.hwDeviceType}
					receiveVerifyAddress={panel.receiveVerifyAddress}
					adminWalletAccountPath={panel.adminWalletAccountPath}
					adminIdDerivationPath={panel.adminIdDerivationPath}
					isAddressesLoading={panel.isAddressesLoading}
					addressRows={panel.addressRows}
					addressRowsLoading={panel.addressRowsLoading}
					addressRowsError={panel.addressRowsError}
					unconfirmedTxRows={panel.unconfirmedTxRows}
					unconfirmedTxsLoading={panel.unconfirmedTxsLoading}
					unconfirmedTxsError={panel.unconfirmedTxsError}
					isWatchOnly={panel.isWatchOnly}
					expandedSection={panel.expandedSection}
					onToggleAddresses={panel.onToggleAddresses}
					onToggleTransactions={panel.onToggleTransactions}
					onOpenSend={panel.onOpenSend}
					onCloseSend={panel.onCloseSend}
					syncStatus={panel.syncStatus}
					isSyncRefreshing={panel.isSyncRefreshing}
					syncError={panel.syncError}
					onRefreshSync={panel.onRefreshSync}
				/>
			</WalletPanel>
		</>
	)
}
