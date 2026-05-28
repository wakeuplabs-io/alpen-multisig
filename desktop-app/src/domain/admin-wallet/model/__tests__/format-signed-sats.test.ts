import assert from 'node:assert/strict'
import { formatSignedSats } from '../format-signed-sats.ts'

assert.equal(formatSignedSats(0), '+0 sats')
assert.equal(formatSignedSats(1_234), '+1,234 sats')
assert.equal(formatSignedSats(-48_250_000), '−48,250,000 sats')
assert.equal(formatSignedSats(NaN), '—')
assert.equal(formatSignedSats(Infinity), '—')

console.log('format-signed-sats: all assertions passed.')
