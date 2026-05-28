import assert from 'node:assert/strict'
import { composeAddressesWithBalance } from '../compose-addresses-with-balance.ts'
import { makeAddress } from '../__fixtures__/make-address.ts'
import { makeUtxo } from '../__fixtures__/make-utxo.ts'

// Addresses with no UTXOs → filtered out (panel lists only addresses *with* balance)
const addrs = [makeAddress({ index: 0 }), makeAddress({ index: 1 })]
const noUtxos = composeAddressesWithBalance(addrs, [])
assert.equal(noUtxos.length, 0)

// Address index 0 with UTXO 125_000_000 → balanceSats: 125_000_000
const utxos = [makeUtxo({ derivationIndex: 0, valueSats: 125_000_000 })]
const withBalance = composeAddressesWithBalance([makeAddress({ index: 0 })], utxos)
assert.equal(withBalance.length, 1)
assert.equal(withBalance[0].balanceSats, 125_000_000)

// isUsed: false but has UTXO → balance reflected (UTXO is truth)
const unusedAddr = makeAddress({ index: 0, isUsed: false })
const result = composeAddressesWithBalance([unusedAddr], utxos)
assert.equal(result.length, 1)
assert.equal(result[0].isUsed, false)
assert.equal(result[0].balanceSats, 125_000_000)

// Mixed: only the address with balance survives
const mixed = composeAddressesWithBalance([makeAddress({ index: 0 }), makeAddress({ index: 1 })], utxos)
assert.equal(mixed.length, 1)
assert.equal(mixed[0].index, 0)

console.log('compose-addresses-with-balance: all assertions passed.')
