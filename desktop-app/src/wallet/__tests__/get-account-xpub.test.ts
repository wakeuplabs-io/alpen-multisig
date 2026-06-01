import assert from 'node:assert/strict'

// Acceptance test: WalletAdapter has optional getAccountXpub, Trezor and Ledger implement it,
// MnemonicAdapter does not.

import { createTrezorAdapter } from '../trezor-adapter.ts'
import { createLedgerAdapter } from '../ledger-adapter.ts'
import { createMnemonicAdapter } from '../mnemonic-adapter.ts'

// AC1: TrezorAdapter has getAccountXpub
const trezor = createTrezorAdapter()
assert.equal(typeof trezor.getAccountXpub, 'function', 'TrezorAdapter must have getAccountXpub')

// AC2: LedgerAdapter has getAccountXpub
const ledger = createLedgerAdapter()
assert.equal(typeof ledger.getAccountXpub, 'function', 'LedgerAdapter must have getAccountXpub')

// AC3: MnemonicAdapter does NOT have getAccountXpub
const mnemonic = createMnemonicAdapter({ mnemonic: 'test test test test test test test test test test test junk' })
assert.equal(mnemonic.getAccountXpub, undefined, 'MnemonicAdapter must not have getAccountXpub')

console.log('get-account-xpub: TrezorAdapter.getAccountXpub OK')
console.log('get-account-xpub: LedgerAdapter.getAccountXpub OK')
console.log('get-account-xpub: MnemonicAdapter.getAccountXpub absent OK')
