import { useState, useEffect } from 'react'
import type { SyncStatusDto, AdminWalletError } from '@/domain/admin-wallet/model/types'
import { relativeTime } from '../model/relative-time'

type SyncChipProps = {
	syncStatus: SyncStatusDto | null
	isRefreshing: boolean
	error: AdminWalletError | null
	onRefresh(): void
	now?: Date
}

function errorMessage(error: AdminWalletError): string {
	if (error.type === 'RpcUnreachable') return 'Sync error: cannot reach Bitcoin node'
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

	let label: string
	if (error != null) {
		label = errorMessage(error)
	} else if (syncStatus?.lastSyncedAt != null) {
		label = relativeTime(syncStatus.lastSyncedAt, currentNow)
	} else {
		label = 'Never synced'
	}

	return (
		<div className="flex items-center gap-2">
			<span className="text-[12px] text-[#9ca3af]">{label}</span>
			<button
				type="button"
				disabled={isRefreshing}
				onClick={onRefresh}
				className="rounded px-2 py-0.5 text-[11px] font-medium text-[#6b7280] hover:bg-[#f3f4f6] disabled:cursor-not-allowed disabled:opacity-50"
			>
				{isRefreshing ? 'Refreshing…' : 'Refresh'}
			</button>
		</div>
	)
}
