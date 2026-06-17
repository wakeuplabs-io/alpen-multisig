import { useState, useEffect } from 'react'
import type { SyncStatusDto, AdminWalletError } from '@/domain/admin-wallet/model/types'
import { relativeTime } from '../model/relative-time'
import { formatSyncProgressLabel } from '../model/format-sync-progress-label'

type SyncChipProps = {
	syncStatus: SyncStatusDto | null
	isRefreshing: boolean
	error: AdminWalletError | null
	onRefresh(): void
	now?: Date
}

function errorMessage(error: AdminWalletError): string {
	if (error.type === 'RpcUnreachable') return 'Sync error: cannot reach Bitcoin node'
	if (error.type === 'ElectrumUnreachable') return 'Sync error: cannot reach Electrum indexer'
	if (error.type === 'RpcAuthFailed') return 'Sync error: RPC auth failed'
	if (error.type === 'Disabled') return 'Sync error: wallet is disabled'
	if (error.type === 'ReadOnly') return 'Sync error: wallet is watch-only'
	return `Sync error: ${error.message}`
}

export function SyncChip({ syncStatus, isRefreshing, error, onRefresh, now }: SyncChipProps) {
	const [, setTick] = useState(0)

	useEffect(() => {
		const interval = setInterval(() => setTick((t) => t + 1), 15000)
		return () => clearInterval(interval)
	}, [])

	const currentNow = now ?? new Date()

	// While syncing past the backend's 3s threshold, progress is present — show "234 / 1,277 (18%)"
	// instead of the static label so the user can see a long sync advancing.
	const progress = isRefreshing ? (syncStatus?.syncProgress ?? null) : null

	let label: string
	if (progress != null) {
		label = formatSyncProgressLabel(progress)
	} else if (error != null) {
		label = errorMessage(error)
	} else if (syncStatus?.lastSyncedAt != null) {
		label = relativeTime(syncStatus.lastSyncedAt, currentNow)
	} else {
		label = 'Never synced'
	}

	return (
		<div className="flex items-center gap-2">
			{isRefreshing && (
				<span
					className="motion-safe:animate-pulse h-1.5 w-1.5 flex-none rounded-full bg-accent"
					aria-hidden="true"
					data-testid={progress != null ? 'e2e-wallet-sync-progress' : undefined}
				/>
			)}
			<span className="text-label text-[#6b7280]" data-testid="e2e-wallet-sync-label">
				{label}
			</span>
			<button
				type="button"
				disabled={isRefreshing}
				onClick={onRefresh}
				data-testid="e2e-wallet-sync-refresh"
				data-syncing={isRefreshing ? 'true' : 'false'}
				className="rounded-md px-2 py-1 text-mono-sm font-medium text-accent transition hover:bg-bg-surface disabled:cursor-not-allowed disabled:opacity-50"
			>
				{isRefreshing ? 'Refreshing…' : 'Refresh'}
			</button>
		</div>
	)
}
