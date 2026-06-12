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

// Progress of an in-flight Electrum sync, counted in sync items (scripts, txids, outpoints).
export type SyncProgressDto = {
	processed: number
	total: number
	percent: number
}

export type SyncStatusDto = {
	tipHeight: number | null
	lastSyncedBlock: number | null
	lastSyncedAt: string | null
	isSyncing: boolean
	lastError: { code: string; message: string } | null
	// Present only while a sync is in flight and has run past the backend's 3s threshold.
	syncProgress: SyncProgressDto | null
}

export type AdminWalletError =
	| { type: 'RpcUnreachable'; message: string }
	| { type: 'ElectrumUnreachable'; message: string }
	| { type: 'RpcAuthFailed'; message: string }
	| { type: 'DescriptorParseError'; message: string }
	| { type: 'SyncIncomplete'; message: string }
	| { type: 'RegtestGuardViolation'; message: string }
	| { type: 'InvalidMnemonic'; message: string }
	| { type: 'Descriptor'; message: string }
	| { type: 'WalletCreation'; message: string }
	| { type: 'Disabled' }
	| { type: 'ReadOnly' }
	// Phase 5 — fee bump (PRD §4.3.3)
	| { type: 'SignerNotAllowedOnNetwork'; message?: string }
	| { type: 'InvalidTxid'; message: string }
	| { type: 'TxNotFound'; message: string }
	| { type: 'TxAlreadyConfirmed'; message: string }
	| { type: 'TxNotReplaceable'; message: string }
	| { type: 'CpfpOutputUnavailable'; message: string }
	| { type: 'FeeTooLow'; message: string }
	| { type: 'FeeRateTooLow'; message: string }
	| { type: 'InvalidFeeRate'; message: string }
	| { type: 'InsufficientFunds'; message: string }
	| { type: 'BuildFailed'; message: string }
	| { type: 'SignFailed'; message: string }
	| { type: 'BroadcastFailed'; message: string }
	// Phase 6 — Send (PRD §4.3.5)
	| { type: 'InvalidAddress'; message: string }
	| { type: 'WrongNetwork'; message: string }
	| { type: 'InvalidAmount'; message: string }
	| { type: 'AmountBelowDust'; message: string }

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

// R1.3 (receive rotation): returns the next unused external (receive) address.
// Idempotent until the address is credited and observed during sync, then rotates.
export function nextAdminWalletReceiveAddress(): Promise<ApiResult<AddressDto>> {
	return tauriCall<AddressDto>('admin_wallet_next_receive_address', {})
}

export function triggerAdminWalletSync(): Promise<ApiResult<void>> {
	return tauriCall<void>('admin_wallet_sync', {})
}

export function getAdminWalletSyncStatus(): Promise<ApiResult<SyncStatusDto>> {
	return tauriCall<SyncStatusDto>('admin_wallet_sync_status', {})
}

// Phase 5 (PRD §4.3.3): how an unconfirmed transaction's fee can be bumped.
// Governance commits use CPFP (a child on the reveal's change); plain sends use RBF.
export type BumpMethodDto = 'rbf' | 'cpfp'

// Phase 5 (PRD §4.3.3): one unconfirmed transaction sent from the Admin Wallet.
export type UnconfirmedTxDto = {
	txid: string
	sentSats: number
	receivedSats: number
	netSats: number
	feeSats: number | null
	feeRateSatPerKvb: number | null
	vsizeVbytes: number
	isRbfSignaling: boolean
	isGovernanceCommit: boolean
	bumpMethod: BumpMethodDto | null
	// commit + reveal package stats — present only for governance commits whose
	// reveal is known to the wallet graph.
	packageFeeSats: number | null
	packageVsizeVbytes: number | null
	packageFeeRateSatPerKvb: number | null
	lastSeenSecs: number | null
}

export type BumpFeeInput = {
	txid: string
	feeRateSatPerKvb: number
}

export type BumpFeeResultDto = {
	// RBF: replacement txid. CPFP: child txid.
	newTxid: string
	targetTxid: string
	// RBF: replacement fee. CPFP: child fee.
	feeSats: number
	// RBF: replacement rate. CPFP: resulting package rate.
	feeRateSatPerKvb: number
	method: BumpMethodDto
}

export function listAdminWalletUnconfirmedTxs(): Promise<ApiResult<UnconfirmedTxDto[]>> {
	return tauriCall<UnconfirmedTxDto[]>('admin_wallet_list_unconfirmed_txs', {})
}

export function bumpAdminWalletFee(input: BumpFeeInput): Promise<ApiResult<BumpFeeResultDto>> {
	return tauriCall<BumpFeeResultDto>('admin_wallet_bump_fee', { input })
}

// Phase 6 (PRD §4.3.5): Send BTC from the Admin Wallet.
export type SendInput = {
	address: string
	// Ignored when drainWallet is true (Max flow).
	amountSats: number
	feeRateSatPerKvb: number
	// Max flow: spend all spendable UTXOs to address, no change output.
	drainWallet?: boolean
}

export type SendResultDto = {
	txid: string
	// Effective amount paid to the destination (the drain value for Max sends).
	amountSats: number
	feeSats: number
	// Realized rate: ceil(fee · 1000 / vsize) in sat/kvB.
	feeRateSatPerKvb: number
	vsizeVbytes: number
	// Change returned to the wallet's internal keychain. 0 for drain builds.
	changeSats: number
	drained: boolean
}

export function sendFromAdminWallet(input: SendInput): Promise<ApiResult<SendResultDto>> {
	return tauriCall<SendResultDto>('admin_wallet_send', { input })
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
