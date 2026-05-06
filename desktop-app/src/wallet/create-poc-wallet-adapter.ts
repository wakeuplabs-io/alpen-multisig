import type { WalletAdapter, WalletVendor, WalletAdapterOptions } from './types'
import { createLedgerPocAdapter } from './ledger-poc-adapter'
import { createMnemonicPocAdapter } from './mnemonic-poc-adapter'
import { createMockPocAdapter } from './mock-poc-adapter'
import { createTrezorPocAdapter } from './trezor-poc-adapter'

export function createWalletAdapter(vendor: WalletVendor, opts: WalletAdapterOptions = {}): WalletAdapter {
	if (vendor === 'mock') return createMockPocAdapter()
	if (vendor === 'ledger') return createLedgerPocAdapter()
	if (vendor === 'trezor') return createTrezorPocAdapter()
	// mnemonic
	if (!opts.mnemonic?.trim()) throw new Error('A BIP39 mnemonic is required for the mnemonic wallet.')
	return createMnemonicPocAdapter({
		mnemonic: opts.mnemonic,
		passphrase: opts.passphrase,
		derivationPath: opts.derivationPath,
	})
}
