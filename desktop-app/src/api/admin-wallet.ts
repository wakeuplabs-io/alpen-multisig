import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'

export type AdminWalletInfo = {
	address: string
	balance_sats: number
}

export type BalanceDto = {
	confirmedSats: number
	unconfirmedSats: number
	totalSats: number
}

export type OutPointDto = {
	txid: string
	vout: number
}

export type KeychainDto = 'External' | 'Internal'

export type UtxoDto = {
	outpoint: OutPointDto
	valueSats: number
	scriptPubkeyHex: string
	keychain: KeychainDto
	derivationIndex: number
	confirmations: number
}

export type AddressDto = {
	index: number
	address: string
	isUsed: boolean
}

export type SyncStatusDto = {
	tipHeight: number | null
	lastSyncedBlock: number | null
	lastSyncedAt: string | null
	isSyncing: boolean
	lastError: { code: string; message: string } | null
}

export type AdminWalletError =
	| { type: 'RpcUnreachable'; message: string }
	| { type: 'RpcAuthFailed'; message: string }
	| { type: 'DescriptorParseError'; message: string }
	| { type: 'SyncIncomplete'; message: string }
	| { type: 'RegtestGuardViolation'; message: string }
	| { type: 'InvalidMnemonic'; message: string }
	| { type: 'Descriptor'; message: string }
	| { type: 'WalletCreation'; message: string }
	| { type: 'Disabled' }
	| { type: 'ReadOnly' }

export function getAdminWalletInfo(): Promise<ApiResult<AdminWalletInfo>> {
	return tauriCall<AdminWalletInfo>('get_admin_wallet_info', {})
}

export function getAdminWalletBalance(): Promise<ApiResult<BalanceDto>> {
	return tauriCall<BalanceDto>('admin_wallet_get_balance', {})
}

export function listAdminWalletUtxos(): Promise<ApiResult<UtxoDto[]>> {
	return tauriCall<UtxoDto[]>('admin_wallet_list_utxos', {})
}

const MAX_PAGE_SIZE = 20

export function listAdminWalletAddresses(
	keychain: KeychainDto = 'External',
	pageIndex = 0,
	pageSize = 20,
): Promise<ApiResult<AddressDto[]>> {
	const clampedPageSize = Math.min(pageSize, MAX_PAGE_SIZE)
	return tauriCall<AddressDto[]>('admin_wallet_list_addresses', {
		keychain,
		pageIndex,
		pageSize: clampedPageSize,
	})
}

export function triggerAdminWalletSync(): Promise<ApiResult<void>> {
	return tauriCall<void>('admin_wallet_sync', {})
}

export function getAdminWalletSyncStatus(): Promise<ApiResult<SyncStatusDto>> {
	return tauriCall<SyncStatusDto>('admin_wallet_sync_status', {})
}

export type WalletSessionInitInput = {
	mnemonic: string
	network?: string
}

export async function walletSessionInit(input: WalletSessionInitInput): Promise<ApiResult<null>> {
	return tauriCall<null>('wallet_session_init', { input })
}

export type WalletSessionInitWatchOnlyInput = {
	xpub: string
	network?: string
	masterFingerprint?: number
	deviceType?: string
}

export function walletSessionInitWatchOnly(input: WalletSessionInitWatchOnlyInput): Promise<ApiResult<null>> {
	return tauriCall<null>('wallet_session_init_watch_only', { input })
}

export type SignerKind = 'hardware' | 'mnemonic' | 'none'

export type AdminWalletCapability = {
	canSign: boolean
	signerKind: SignerKind
	reason?: string
}

export function getAdminWalletCanSign(): Promise<ApiResult<AdminWalletCapability>> {
	return tauriCall<AdminWalletCapability>('admin_wallet_can_sign', {})
}
