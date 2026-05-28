import type { SyncStatusDto } from '@/api/admin-wallet'

export function makeSyncStatus(overrides?: Partial<SyncStatusDto>): SyncStatusDto {
	return {
		tipHeight: 100,
		lastSyncedBlock: 100,
		lastSyncedAt: '2024-01-01T12:00:00Z',
		isSyncing: false,
		lastError: null,
		...overrides,
	}
}
