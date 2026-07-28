import { SettingsGearIcon } from '@/assets/icons'
import type { ConnectionMode, LocalNodeStatus } from '@/domain/node-config/model/node-config.types'

type Props = {
	mode: ConnectionMode
	localNodeStatus: LocalNodeStatus | null
	onClick: () => void
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

	return (
		<button
			type="button"
			aria-label={`Network: ${MODE_LABELS[mode]}, ${isConnected ? 'connected' : 'connection issue'}. Open node settings.`}
			onClick={onClick}
			className={`inline-flex items-center gap-1.5 rounded-lg border bg-white px-2.5 py-1.25 text-label font-medium text-[#6b7280] transition hover:bg-[#f3f4f6] hover:text-[#374151] ${
				isConnected ? 'border-[#e5e7eb]' : 'border-[#d1d5db] text-emphasis-soft hover:text-emphasis'
			}`}
		>
			<span
				className={`h-2 w-2 shrink-0 rounded-full ${isConnected ? 'bg-emerald-500' : 'bg-emphasis-soft'}`}
				aria-hidden
			/>
			{MODE_LABELS[mode]}
			<SettingsGearIcon width={14} height={14} className="text-current" />
		</button>
	)
}
