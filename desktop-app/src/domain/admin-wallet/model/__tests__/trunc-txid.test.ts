import assert from 'node:assert/strict'
import { truncTxid } from '../trunc-txid.ts'

assert.equal(truncTxid(''), '')
assert.equal(truncTxid('1234567890123456'), '1234567890123456')
assert.equal(truncTxid('a7f3c9e2b1d4f8a3c6e9b2d5f8a1c4e7b0d3f6a9c2e5b8d1f4a7c0e3b6d9f2a5'), 'a7f3c9e2…d9f2a5')

console.log('trunc-txid: all assertions passed.')
