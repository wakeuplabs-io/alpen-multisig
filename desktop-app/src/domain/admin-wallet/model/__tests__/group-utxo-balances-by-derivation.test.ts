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

console.log('group-utxo-balances-by-derivation: all assertions passed.')
