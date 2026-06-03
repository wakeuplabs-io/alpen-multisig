import assert from 'node:assert/strict'
import { formatUnconfirmedBalanceLine } from '../format-unconfirmed-balance-line.ts'

assert.equal(formatUnconfirmedBalanceLine(0), null)
assert.equal(formatUnconfirmedBalanceLine(1_234), '+1,234 sats unconfirmed')
assert.equal(formatUnconfirmedBalanceLine(-48_250_000), '−48,250,000 sats unconfirmed')
assert.equal(formatUnconfirmedBalanceLine(NaN), '— unconfirmed')
assert.equal(formatUnconfirmedBalanceLine(Infinity), '— unconfirmed')

console.log('format-unconfirmed-balance-line: all assertions passed.')
