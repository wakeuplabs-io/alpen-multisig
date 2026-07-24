import assert from 'node:assert/strict'
import { denominateSats, formatDenominatedBalance, toggleDenomination } from '../balance-denomination.ts'

// The unit label is the toggle: clicking it always lands on the other denomination.
assert.equal(toggleDenomination('BTC'), 'sats')
assert.equal(toggleDenomination('sats'), 'BTC')
assert.equal(toggleDenomination(toggleDenomination('BTC')), 'BTC')

assert.deepEqual(denominateSats(123_456, 'BTC'), { amount: '0.00123456', unit: 'BTC' })
assert.deepEqual(denominateSats(123_456, 'sats'), { amount: '123,456', unit: 'sats' })
assert.deepEqual(denominateSats(0, 'sats'), { amount: '0', unit: 'sats' })

// Non-finite balances never render a bogus number in either denomination.
assert.deepEqual(denominateSats(NaN, 'sats'), { amount: '—', unit: 'sats' })
assert.deepEqual(denominateSats(NaN, 'BTC'), { amount: '—', unit: 'BTC' })

assert.equal(formatDenominatedBalance(denominateSats(100_000_000, 'BTC')), '1.000000 BTC')
assert.equal(formatDenominatedBalance(denominateSats(100_000_000, 'sats')), '100,000,000 sats')

console.log('balance-denomination: all assertions passed.')
