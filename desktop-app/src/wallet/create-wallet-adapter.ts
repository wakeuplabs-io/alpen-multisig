import type { WalletAdapter, WalletVendor, WalletAdapterOptions } from './types'
import { createHwAdapter } from './hw-adapter'
import { createMnemonicAdapter } from './mnemonic-adapter'
import { createMockAdapter } from './mock-adapter'

export function createWalletAdapter(vendor: WalletVendor, opts: WalletAdapterOptions = {}): WalletAdapter {
	if (vendor === 'mock') return createMockAdapter()
	if (vendor === 'ledger') return createHwAdapter('ledger')
	if (vendor === 'trezor') return createHwAdapter('trezor')
	// mnemonic
	if (!opts.mnemonic?.trim()) throw new Error('A BIP39 mnemonic is required for the mnemonic wallet.')
	return createMnemonicAdapter({
		mnemonic: opts.mnemonic,
		passphrase: opts.passphrase,
		derivationPath: opts.derivationPath,
	})
}
