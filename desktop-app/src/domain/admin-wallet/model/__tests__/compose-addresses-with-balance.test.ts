import assert from 'node:assert/strict'
import { composeAddressesWithBalance } from '../compose-addresses-with-balance.ts'
import { makeAddress } from '../__fixtures__/make-address.ts'
import { makeUtxo } from '../__fixtures__/make-utxo.ts'

// Addresses with no UTXOs → all balanceSats 0
const addrs = [makeAddress({ index: 0 }), makeAddress({ index: 1 })]
const noUtxos = composeAddressesWithBalance(addrs, [])
assert.equal(noUtxos[0].balanceSats, 0)
assert.equal(noUtxos[1].balanceSats, 0)

// Address index 0 with UTXO 125_000_000 → balanceSats: 125_000_000
const utxos = [makeUtxo({ derivationIndex: 0, valueSats: 125_000_000 })]
const withBalance = composeAddressesWithBalance([makeAddress({ index: 0 })], utxos)
assert.equal(withBalance[0].balanceSats, 125_000_000)

// isUsed: false but has UTXO → balance reflected (UTXO is truth)
const unusedAddr = makeAddress({ index: 0, isUsed: false })
const result = composeAddressesWithBalance([unusedAddr], utxos)
assert.equal(result[0].isUsed, false)
assert.equal(result[0].balanceSats, 125_000_000)

console.log('compose-addresses-with-balance: all assertions passed.')
