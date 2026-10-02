import { SettingsGearIcon } from '@/assets/icons'
import type { ConnectionMode, LocalNodeStatus } from '@/domain/node-config/model/node-config.types'

type Props = {
	mode: ConnectionMode
	localNodeStatus: LocalNodeStatus | null
	/** Opens node settings. Omit it for the read-only indicator shown after sign-in. */
	onClick?: () => void
}

const MODE_LABELS: Record<ConnectionMode, string> = {
	local: 'Local',
	trusted: 'Trusted',
	custom: 'Custom',
}

export function NetworkStatusPill({ mode, localNodeStatus, onClick }: Props) {
	const isConnected =
		localNodeStatus !== null &&
		localNodeStatus.strataReachable &&
		localNodeStatus.btcReachable &&
		localNodeStatus.orchestratorReachable
	const statusLabel = `Network: ${MODE_LABELS[mode]}, ${isConnected ? 'connected' : 'connection issue'}`
	const dot = (
		<span
			className={`h-2 w-2 shrink-0 rounded-full ${isConnected ? 'bg-emerald-500' : 'bg-emphasis-soft'}`}
			aria-hidden
		/>
	)

	if (onClick === undefined) {
		return (
			<span
				role="status"
				aria-label={statusLabel}
				data-testid="network-status-pill"
				className={`inline-flex items-center gap-1.5 rounded-lg border bg-white px-2.5 py-1.25 text-label font-medium ${
					isConnected ? 'border-[#e5e7eb] text-[#6b7280]' : 'border-[#d1d5db] text-emphasis-soft'
				}`}
			>
				{dot}
				{MODE_LABELS[mode]}
			</span>
		)
	}

	return (
		<button
			type="button"
			aria-label={`${statusLabel}. Open node settings.`}
			data-testid="network-status-pill"
			onClick={onClick}
			className={`inline-flex items-center gap-1.5 rounded-lg border bg-white px-2.5 py-1.25 text-label font-medium text-[#6b7280] transition hover:bg-[#f3f4f6] hover:text-[#374151] ${
				isConnected ? 'border-[#e5e7eb]' : 'border-[#d1d5db] text-emphasis-soft hover:text-emphasis'
			}`}
		>
			{dot}
			{MODE_LABELS[mode]}
			<SettingsGearIcon width={14} height={14} className="text-current" />
		</button>
	)
}
