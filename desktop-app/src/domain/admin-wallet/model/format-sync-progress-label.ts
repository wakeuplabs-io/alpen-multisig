import type { SyncProgressDto } from '@/domain/admin-wallet/model/types'

// Formats sync progress into a human label, e.g. "Syncing 234 / 1,277 blocks (18%)".
// Thousands grouping uses en-US so the label stays stable across locales.
export function formatSyncProgressLabel(progress: SyncProgressDto): string {
	const processed = progress.processedBlocks.toLocaleString('en-US')
	const total = progress.totalBlocks.toLocaleString('en-US')
	return `Syncing ${processed} / ${total} blocks (${progress.percent}%)`
}
