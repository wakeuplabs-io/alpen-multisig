import assert from 'node:assert/strict'
import { composeAddressesWithBalance } from '../compose-addresses-with-balance.ts'
import { makeAddress } from '../__fixtures__/make-address.ts'
import { makeUtxo } from '../__fixtures__/make-utxo.ts'

const addrs = [makeAddress({ index: 0 }), makeAddress({ index: 1 })]
const noUtxos = composeAddressesWithBalance(addrs, [])
assert.equal(noUtxos.length, 0)

const confirmedOnly = composeAddressesWithBalance(
	[makeAddress({ index: 0 })],
	[makeUtxo({ derivationIndex: 0, valueSats: 125_000_000, confirmations: 6 })],
)
assert.equal(confirmedOnly.length, 1)
assert.equal(confirmedOnly[0].confirmedSats, 125_000_000)
assert.equal(confirmedOnly[0].unconfirmedSats, 0)
assert.equal(confirmedOnly[0].balanceSats, 125_000_000)

const unconfirmedOnly = composeAddressesWithBalance(
	[makeAddress({ index: 0 })],
	[makeUtxo({ derivationIndex: 0, valueSats: 50_000, confirmations: 0 })],
)
assert.equal(unconfirmedOnly.length, 1)
assert.equal(unconfirmedOnly[0].confirmedSats, 0)
assert.equal(unconfirmedOnly[0].unconfirmedSats, 50_000)
assert.equal(unconfirmedOnly[0].balanceSats, 50_000)

const mixed = composeAddressesWithBalance(
	[makeAddress({ index: 0 })],
	[
		makeUtxo({ derivationIndex: 0, valueSats: 100_000, confirmations: 1 }),
		makeUtxo({ derivationIndex: 0, valueSats: 25_000, confirmations: 0 }),
	],
)
assert.equal(mixed[0].confirmedSats, 100_000)
assert.equal(mixed[0].unconfirmedSats, 25_000)
assert.equal(mixed[0].balanceSats, 125_000)

const unusedAddr = makeAddress({ index: 0, isUsed: false })
const withBalance = composeAddressesWithBalance(
	[unusedAddr],
	[makeUtxo({ derivationIndex: 0, valueSats: 125_000_000 })],
)
assert.equal(withBalance.length, 1)
assert.equal(withBalance[0].isUsed, false)
assert.equal(withBalance[0].balanceSats, 125_000_000)

const twoIndices = composeAddressesWithBalance(
	[makeAddress({ index: 0 }), makeAddress({ index: 1 })],
	[makeUtxo({ derivationIndex: 0, valueSats: 10_000 }), makeUtxo({ derivationIndex: 1, valueSats: 20_000 })],
)
assert.equal(twoIndices.length, 2)
assert.equal(twoIndices[0].index, 0)
assert.equal(twoIndices[1].index, 1)
assert.equal(twoIndices[1].confirmedSats, 20_000)

const internalIgnored = composeAddressesWithBalance(
	[makeAddress({ index: 0 })],
	[makeUtxo({ derivationIndex: 0, valueSats: 99_000, keychain: 'Internal' })],
)
assert.equal(internalIgnored.length, 0)

console.log('compose-addresses-with-balance: all assertions passed.')
