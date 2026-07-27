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
	/**
	 * Device-accurate verification value for HW sessions: the same address re-encoded with the
	 * HRP the connected device shows (tb1… on a Testnet app, bc1… on a Bitcoin app), so the
	 * on-device comparison is exact. Absent for software sessions and address listings.
	 */
	verifyAddress?: string
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

// Phase 6 P6.2 (PRD §4.3.5.1): inline destination validation.
export type SendAddressInvalidReason = 'invalid-address' | 'wrong-network'

// Validation failures are a successful IPC result (form states, not faults);
// the exact PRD copy is rendered from reason + expectedNetwork.
export type SendAddressValidationDto = {
	isValid: boolean
	reason: SendAddressInvalidReason | null
	expectedNetwork: string
}

export function validateSendAddress(address: string): Promise<ApiResult<SendAddressValidationDto>> {
	return tauriCall<SendAddressValidationDto>('admin_wallet_validate_send_address', { address })
}

// Phase 6 P6.3 (PRD §4.3.5.2/§4.3.5.3): dry-run estimate — never signed, never
// broadcast. Powers the fee preview, the Max button (drainWallet: true), and
// the "Insufficient funds" boundary before Confirm.
export type SendEstimateDto = {
	// Effective amount: the input amount, or the computed max for drain runs.
	amountSats: number
	feeSats: number
	feeRateSatPerKvb: number
	vsizeVbytes: number
	// Change returned to the wallet's internal keychain. 0 for drain builds.
	changeSats: number
	// Max spendable to this destination at this rate (drain dry-run).
	maxAmountSats: number
}

export function estimateSend(input: SendInput): Promise<ApiResult<SendEstimateDto>> {
	return tauriCall<SendEstimateDto>('admin_wallet_estimate_send', { input })
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

// Re-exported from the domain model so presentational components can type their
// props without importing this API boundary (see admin-wallet architecture rule).
export type { HwDeviceType, VerifyScriptType } from '@/domain/admin-wallet/model/hw-device'
import type { HwDeviceType, VerifyScriptType } from '@/domain/admin-wallet/model/hw-device'

export type AdminWalletCapability = {
	canSign: boolean
	signerKind: SignerKind
	reason?: string
	/** Connected device for verify-on-device dispatch (PRD §4.2 / §4.3.4.2); null when not HW. */
	deviceType: HwDeviceType | null
	/** Active session network token (regtest | testnet | signet | bitcoin). */
	network: string
	/**
	 * BIP-86 account derivation path used for this device/network (e.g. m/86'/0'/73' for Trezor,
	 * m/86'/1'/73' for Ledger on test nets). The receive verify-on-device path is built from this
	 * so it matches the device's coin type. Null for software / no signer.
	 */
	adminWalletAccountPath: string | null
}

export function getAdminWalletCanSign(): Promise<ApiResult<AdminWalletCapability>> {
	return tauriCall<AdminWalletCapability>('admin_wallet_can_sign', {})
}

// Phase 8 (PRD §4.2 / §4.3.4.2): confirm an address on the connected device screen.
export type VerifyAddressOnDeviceInput = {
	derivationPath: string
	deviceType: HwDeviceType
	scriptType: VerifyScriptType
	network?: string
}

/**
 * Asks the connected hardware device to display the address at derivationPath so
 * the signer can compare it on-screen. Resolves with the exact address the device
 * rendered; ApiResult error on device rejection or timeout.
 */
export function verifyAddressOnDevice(input: VerifyAddressOnDeviceInput): Promise<ApiResult<string>> {
	return tauriCall<string>('verify_address_on_device', {
		derivationPath: input.derivationPath,
		deviceType: input.deviceType,
		scriptType: input.scriptType,
		network: input.network,
	})
}
