import assert from 'node:assert/strict'
import { groupUtxosByDerivation } from '../group-utxos-by-derivation.ts'
import { makeUtxo } from '../__fixtures__/make-utxo.ts'

// Empty → empty map
const emptyResult = groupUtxosByDerivation([])
assert.equal(emptyResult.size, 0)

// Two UTXOs same index → sum
const twoSameIndex = [
	makeUtxo({ derivationIndex: 0, valueSats: 50_000 }),
	makeUtxo({ derivationIndex: 0, valueSats: 75_000 }),
]
const sumResult = groupUtxosByDerivation(twoSameIndex)
assert.equal(sumResult.get(0), 125_000)

// Internal excluded by default
const withInternal = [
	makeUtxo({ derivationIndex: 1, valueSats: 100_000, keychain: 'External' }),
	makeUtxo({ derivationIndex: 2, valueSats: 200_000, keychain: 'Internal' }),
]
const externalOnly = groupUtxosByDerivation(withInternal)
assert.equal(externalOnly.has(1), true)
assert.equal(externalOnly.has(2), false)

// includeInternal: true → both included
const bothResult = groupUtxosByDerivation(withInternal, { includeInternal: true })
assert.equal(bothResult.has(1), true)
assert.equal(bothResult.has(2), true)

console.log('group-utxos-by-derivation: all assertions passed.')
