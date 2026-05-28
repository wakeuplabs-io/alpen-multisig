import { useNavigate } from 'react-router-dom'
import { useBlockPayouts } from '@/domain/block-payouts/hooks/use-block-payouts'
import { BlockPayoutsDashboard } from '@/domain/block-payouts/components/block-payouts-dashboard'
import { ScreenShell } from '@/screens/screen-shell'
import { LogOutMutedIcon, LogOutRedIcon, ShieldPurpleIcon } from '@/assets/icons'
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
			headerContent={
				<>
					<span className="inline-flex items-center gap-1.5 rounded-md border border-[#e4dfff] bg-[#f5f3ff] px-2.5 py-1 text-[12px] font-medium text-[#7c6fcd]">
						<ShieldPurpleIcon width={12} height={12} className="block shrink-0" />
						Payout Administrator
					</span>

					<button
						type="button"
						className="group/disconnect inline-flex items-center gap-1.5 rounded-lg border border-[#e5e7eb] bg-white px-2.5 py-1.25 text-[12px] font-medium text-[#6b7280] transition hover:border-[#fca5a5] hover:bg-[#fef2f2] hover:text-[#b91c1c]"
						onClick={() => void handleDisconnect()}
					>
						<span className="relative inline-flex h-3 w-3 shrink-0">
							<LogOutMutedIcon
								width={12}
								height={12}
								className="absolute left-0 top-0 transition-opacity group-hover/disconnect:opacity-0"
							/>
							<LogOutRedIcon
								width={12}
								height={12}
								className="absolute left-0 top-0 opacity-0 transition-opacity group-hover/disconnect:opacity-100"
							/>
						</span>
						Disconnect
					</button>
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
