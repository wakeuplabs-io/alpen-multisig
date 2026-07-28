import { useNavigate } from 'react-router-dom'
import { DisconnectButton } from '@/components/disconnect-button'
import { useBlockPayouts } from '@/domain/block-payouts/hooks/use-block-payouts'
import { BlockPayoutsDashboard } from '@/domain/block-payouts/components/block-payouts-dashboard'
import { ScreenShell } from '@/screens/screen-shell'
import { ShieldAccentIcon } from '@/assets/icons'
import { useSession } from '@/hooks/use-session'

export function BlockPayoutsScreen() {
	const hook = useBlockPayouts()
	const navigate = useNavigate()
	const { disconnectSession } = useSession()

	async function handleDisconnect() {
		await disconnectSession()
		navigate('/')
	}

	return (
		<ScreenShell
			authorityBadge={
				<span className="inline-flex items-center gap-1.5 rounded-md border border-accent-border bg-bg-surface px-2.5 py-1 text-label font-medium text-accent-hover">
					<ShieldAccentIcon width={12} height={12} className="block shrink-0" />
					Payout Administrator
				</span>
			}
			headerContent={
				<>
					<DisconnectButton onClick={() => void handleDisconnect()} />
				</>
			}
		>
			<BlockPayoutsDashboard
				activeTab={hook.activeTab}
				pendingTxs={hook.pendingTxs}
				pastTxs={hook.pastTxs}
				hasConflicts={hook.hasConflicts}
				activeModal={hook.activeModal}
				toast={hook.toast}
				onTabChange={hook.setActiveTab}
				openSignModal={hook.openSignModal}
				openPasteModal={hook.openPasteModal}
				openCreateModal={hook.openCreateModal}
				closeModal={hook.closeModal}
				handleSign={hook.handleSign}
				handlePasteSignatures={hook.handlePasteSignatures}
				handleExport={hook.handleExport}
				handleCopySignatures={hook.handleCopySignatures}
				handleRebroadcast={hook.handleRebroadcast}
				handleCopyRawTx={hook.handleCopyRawTx}
				handleCreateTx={hook.handleCreateTx}
			/>
		</ScreenShell>
	)
}
