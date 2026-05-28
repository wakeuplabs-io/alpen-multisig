import assert from 'node:assert/strict'
import { formatBtcFromSats } from '../format-btc-from-sats.ts'

assert.equal(formatBtcFromSats(0), '0.00000000')
assert.equal(formatBtcFromSats(99_999_999), '0.99999999')
assert.equal(formatBtcFromSats(100_000_000), '1.000000')
assert.equal(formatBtcFromSats(10_000_000_000), '100.0000')
assert.equal(formatBtcFromSats(NaN), '—')
assert.equal(formatBtcFromSats(-1_234), '-0.00001234')

console.log('format-btc-from-sats: all assertions passed.')
