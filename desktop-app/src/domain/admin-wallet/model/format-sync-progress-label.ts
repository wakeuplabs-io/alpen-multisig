import type { SyncProgressDto } from '@/domain/admin-wallet/model/types'

// Formats sync progress into a human label, e.g. "Syncing 234 / 1,277 (18%)".
// Counts are Electrum sync items (scripts, txids, outpoints), so the label stays unitless.
// Thousands grouping uses en-US so the label stays stable across locales.
export function formatSyncProgressLabel(progress: SyncProgressDto): string {
	const processed = progress.processed.toLocaleString('en-US')
	const total = progress.total.toLocaleString('en-US')
	return `Syncing ${processed} / ${total} (${progress.percent}%)`
}
