import assert from 'node:assert/strict'
import { groupUtxoBalancesByDerivation } from '../group-utxo-balances-by-derivation.ts'
import { makeUtxo } from '../__fixtures__/make-utxo.ts'

const empty = groupUtxoBalancesByDerivation([])
assert.equal(empty.size, 0)

const confirmed = groupUtxoBalancesByDerivation([makeUtxo({ derivationIndex: 2, valueSats: 100, confirmations: 3 })])
assert.equal(confirmed.get(2)?.confirmedSats, 100)
assert.equal(confirmed.get(2)?.unconfirmedSats, 0)

const unconfirmed = groupUtxoBalancesByDerivation([makeUtxo({ derivationIndex: 1, valueSats: 50, confirmations: 0 })])
assert.equal(unconfirmed.get(1)?.unconfirmedSats, 50)

// Keychain filter: each call returns only the requested keychain's UTXOs, so external 0 and
// internal 0 (different addresses, same index) never merge into one bucket.
const mixedKeychains = [
	makeUtxo({ derivationIndex: 0, valueSats: 100, keychain: 'External' }),
	makeUtxo({ derivationIndex: 0, valueSats: 40, keychain: 'Internal' }),
]
const externalOnly = groupUtxoBalancesByDerivation(mixedKeychains, 'External')
assert.equal(externalOnly.get(0)?.confirmedSats, 100)
const internalOnly = groupUtxoBalancesByDerivation(mixedKeychains, 'Internal')
assert.equal(internalOnly.get(0)?.confirmedSats, 40)

// Default keychain is External
const defaulted = groupUtxoBalancesByDerivation(mixedKeychains)
assert.equal(defaulted.get(0)?.confirmedSats, 100)

console.log('group-utxo-balances-by-derivation: all assertions passed.')
