// Domain-level type re-exports.
// Components must import domain types from here, not from @/api/admin-wallet directly.
export type {
	AdminWalletError,
	SyncStatusDto,
	SyncProgressDto,
	KeychainDto,
	BumpMethodDto,
	SendInput,
	SendResultDto,
} from '@/api/admin-wallet'
