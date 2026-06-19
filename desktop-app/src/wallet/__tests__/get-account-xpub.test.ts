import assert from 'node:assert/strict'

// Acceptance test: WalletAdapter has optional getAccountXpub, Trezor and Ledger implement it,
// MnemonicAdapter does not.

import { createHwAdapter } from '../hw-adapter.ts'
import { createMnemonicAdapter } from '../mnemonic-adapter.ts'

// AC1: HwAdapter (trezor) has getAccountXpub
const trezor = createHwAdapter('trezor')
assert.equal(typeof trezor.getAccountXpub, 'function', 'HwAdapter(trezor) must have getAccountXpub')

// AC2: HwAdapter (ledger) has getAccountXpub
const ledger = createHwAdapter('ledger')
assert.equal(typeof ledger.getAccountXpub, 'function', 'HwAdapter(ledger) must have getAccountXpub')

// AC3: MnemonicAdapter does NOT have getAccountXpub
const mnemonic = createMnemonicAdapter({ mnemonic: 'test test test test test test test test test test test junk' })
assert.equal(mnemonic.getAccountXpub, undefined, 'MnemonicAdapter must not have getAccountXpub')

console.log('get-account-xpub: HwAdapter(trezor).getAccountXpub OK')
console.log('get-account-xpub: HwAdapter(ledger).getAccountXpub OK')
console.log('get-account-xpub: MnemonicAdapter.getAccountXpub absent OK')
