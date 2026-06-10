import assert from 'node:assert/strict'
import { composeAddressesWithBalance } from '../compose-addresses-with-balance.ts'
import { makeAddress } from '../__fixtures__/make-address.ts'
import { makeUtxo } from '../__fixtures__/make-utxo.ts'

const noInternal = { internal: [] }

const addrs = [makeAddress({ index: 0 }), makeAddress({ index: 1 })]
const noUtxos = composeAddressesWithBalance({ external: addrs, ...noInternal }, [])
assert.equal(noUtxos.length, 0)

const confirmedOnly = composeAddressesWithBalance({ external: [makeAddress({ index: 0 })], ...noInternal }, [
	makeUtxo({ derivationIndex: 0, valueSats: 125_000_000, confirmations: 6 }),
])
assert.equal(confirmedOnly.length, 1)
assert.equal(confirmedOnly[0].confirmedSats, 125_000_000)
assert.equal(confirmedOnly[0].unconfirmedSats, 0)
assert.equal(confirmedOnly[0].balanceSats, 125_000_000)
assert.equal(confirmedOnly[0].keychain, 'External')

const unconfirmedOnly = composeAddressesWithBalance({ external: [makeAddress({ index: 0 })], ...noInternal }, [
	makeUtxo({ derivationIndex: 0, valueSats: 50_000, confirmations: 0 }),
])
assert.equal(unconfirmedOnly.length, 1)
assert.equal(unconfirmedOnly[0].confirmedSats, 0)
assert.equal(unconfirmedOnly[0].unconfirmedSats, 50_000)
assert.equal(unconfirmedOnly[0].balanceSats, 50_000)

const mixed = composeAddressesWithBalance({ external: [makeAddress({ index: 0 })], ...noInternal }, [
	makeUtxo({ derivationIndex: 0, valueSats: 100_000, confirmations: 1 }),
	makeUtxo({ derivationIndex: 0, valueSats: 25_000, confirmations: 0 }),
])
assert.equal(mixed[0].confirmedSats, 100_000)
assert.equal(mixed[0].unconfirmedSats, 25_000)
assert.equal(mixed[0].balanceSats, 125_000)

const unusedAddr = makeAddress({ index: 0, isUsed: false })
const withBalance = composeAddressesWithBalance({ external: [unusedAddr], ...noInternal }, [
	makeUtxo({ derivationIndex: 0, valueSats: 125_000_000 }),
])
assert.equal(withBalance.length, 1)
assert.equal(withBalance[0].isUsed, false)
assert.equal(withBalance[0].balanceSats, 125_000_000)

const twoIndices = composeAddressesWithBalance(
	{ external: [makeAddress({ index: 0 }), makeAddress({ index: 1 })], ...noInternal },
	[makeUtxo({ derivationIndex: 0, valueSats: 10_000 }), makeUtxo({ derivationIndex: 1, valueSats: 20_000 })],
)
assert.equal(twoIndices.length, 2)
assert.equal(twoIndices[0].index, 0)
assert.equal(twoIndices[1].index, 1)
assert.equal(twoIndices[1].confirmedSats, 20_000)

// Regression (change invisibility bug): after a spend the whole balance sits in a change UTXO.
// The internal address holding it MUST appear — previously the list showed "No addresses with
// balance" while the header balance was non-zero.
const changeAddr = makeAddress({ index: 0, address: 'bcrt1p-change-0' })
const changeOnly = composeAddressesWithBalance(
	{ external: [makeAddress({ index: 0 }), makeAddress({ index: 1 })], internal: [changeAddr] },
	[makeUtxo({ derivationIndex: 0, valueSats: 99_998_991, keychain: 'Internal', confirmations: 2 })],
)
assert.equal(changeOnly.length, 1)
assert.equal(changeOnly[0].keychain, 'Internal')
assert.equal(changeOnly[0].address, 'bcrt1p-change-0')
assert.equal(changeOnly[0].confirmedSats, 99_998_991)

// Index collision across keychains: external 0 and internal 0 hold separate balances and
// produce two distinct rows (no merging by bare index).
const collision = composeAddressesWithBalance(
	{
		external: [makeAddress({ index: 0, address: 'bcrt1p-ext-0' })],
		internal: [makeAddress({ index: 0, address: 'bcrt1p-int-0' })],
	},
	[
		makeUtxo({ derivationIndex: 0, valueSats: 70_000, keychain: 'External' }),
		makeUtxo({ derivationIndex: 0, valueSats: 30_000, keychain: 'Internal' }),
	],
)
assert.equal(collision.length, 2)
const extRow = collision.find((r) => r.keychain === 'External')
const intRow = collision.find((r) => r.keychain === 'Internal')
assert.equal(extRow?.balanceSats, 70_000)
assert.equal(extRow?.address, 'bcrt1p-ext-0')
assert.equal(intRow?.balanceSats, 30_000)
assert.equal(intRow?.address, 'bcrt1p-int-0')

// Internal UTXO without a matching internal address window entry → not listed (window cap),
// but external rows are unaffected.
const orphanInternal = composeAddressesWithBalance({ external: [makeAddress({ index: 0 })], internal: [] }, [
	makeUtxo({ derivationIndex: 0, valueSats: 99_000, keychain: 'Internal' }),
])
assert.equal(orphanInternal.length, 0)

console.log('compose-addresses-with-balance: all assertions passed.')
