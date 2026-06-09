import assert from 'node:assert/strict'
import { initAdminWalletForAdapter } from '../session-provider-vendor-branch.ts'
import type { WalletAdapter } from '@/wallet/types.ts'
import type { ApiResult } from '@/types'

// Test Budget: 3 behaviors x 2 = 6 max. Using 3 focused tests.

const stubSignSighash = async () => ({
	publicKeyHex: '',
	signatureHex: '',
	signatureFormat: 'raw-ecdsa' as const,
})
const stubConnect = async () => ({ deviceLabel: 'stub', derivationPath: "m/44'/0'/0'" })
const stubDisconnect = async () => {}

// --- Behavior 1: HW vendor (trezor/ledger) calls getAccountXpub then walletSessionInitWatchOnly ---

const trezorAdapter: WalletAdapter = {
	vendor: 'trezor',
	supportsSighashSigning: true,
	connect: stubConnect,
	disconnect: stubDisconnect,
	signSighash: stubSignSighash,
	getAccountXpub: async () => 'xpub_from_device',
	getMasterFingerprint: async () => 0xabcd1234,
}

let watchOnlyCalledWith: {
	xpub: string
	masterFingerprint?: number
	deviceType?: string
} | null = null
const mockWalletSessionInitWatchOnly = async (input: {
	xpub: string
	masterFingerprint?: number
	deviceType?: string
}): Promise<ApiResult<null>> => {
	watchOnlyCalledWith = input
	return { ok: true, data: null }
}
const mockWalletSessionInit = async (_input: { mnemonic: string }): Promise<ApiResult<null>> => {
	throw new Error('walletSessionInit must not be called for HW vendors')
}

await initAdminWalletForAdapter(trezorAdapter, mockWalletSessionInitWatchOnly, mockWalletSessionInit)
assert.deepEqual(watchOnlyCalledWith, {
	xpub: 'xpub_from_device',
	masterFingerprint: 0xabcd1234,
	deviceType: 'trezor',
})
console.log('session-provider-vendor-branch: trezor calls walletSessionInitWatchOnly OK')

// ledger same behavior
watchOnlyCalledWith = null
const ledgerAdapter: WalletAdapter = {
	vendor: 'ledger',
	supportsSighashSigning: true,
	connect: stubConnect,
	disconnect: stubDisconnect,
	signSighash: stubSignSighash,
	getAccountXpub: async () => 'xpub_ledger',
	getMasterFingerprint: async () => 0xdeadbeef,
}
await initAdminWalletForAdapter(ledgerAdapter, mockWalletSessionInitWatchOnly, mockWalletSessionInit)
assert.deepEqual(watchOnlyCalledWith, {
	xpub: 'xpub_ledger',
	masterFingerprint: 0xdeadbeef,
	deviceType: 'ledger',
})
console.log('session-provider-vendor-branch: ledger calls walletSessionInitWatchOnly OK')

// --- Behavior 2: mnemonic vendor calls walletSessionInit (regression) ---

let mnemonicCalledWith: { mnemonic: string } | null = null
const mockWalletSessionInitForMnemonic = async (input: { mnemonic: string }): Promise<ApiResult<null>> => {
	mnemonicCalledWith = input
	return { ok: true, data: null }
}
const mockWatchOnlyForMnemonic = async (_input: { xpub: string }): Promise<ApiResult<null>> => {
	throw new Error('walletSessionInitWatchOnly must not be called for mnemonic')
}

const mnemonicAdapter: WalletAdapter & { getMnemonic(): string } = {
	vendor: 'mnemonic',
	supportsSighashSigning: true,
	connect: stubConnect,
	disconnect: stubDisconnect,
	signSighash: stubSignSighash,
	getMnemonic: () => 'word1 word2 word3',
}
await initAdminWalletForAdapter(mnemonicAdapter, mockWatchOnlyForMnemonic, mockWalletSessionInitForMnemonic)
assert.deepEqual(mnemonicCalledWith, { mnemonic: 'word1 word2 word3' }, 'mnemonic: walletSessionInit called')
console.log('session-provider-vendor-branch: mnemonic calls walletSessionInit OK')

// --- Behavior 3: mock vendor calls neither ---

let watchOnlyForMock = false
let sessionInitForMock = false
const mockAdapter: WalletAdapter = {
	vendor: 'mock',
	supportsSighashSigning: false,
	connect: stubConnect,
	disconnect: stubDisconnect,
	signSighash: stubSignSighash,
}
await initAdminWalletForAdapter(
	mockAdapter,
	async () => {
		watchOnlyForMock = true
		return { ok: true, data: null }
	},
	async () => {
		sessionInitForMock = true
		return { ok: true, data: null }
	},
)
assert.equal(watchOnlyForMock, false, 'mock: walletSessionInitWatchOnly not called')
assert.equal(sessionInitForMock, false, 'mock: walletSessionInit not called')
console.log('session-provider-vendor-branch: mock vendor skips both inits OK')
