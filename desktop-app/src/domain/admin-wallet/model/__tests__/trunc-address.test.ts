import assert from 'node:assert/strict'
import { truncAddress } from '../trunc-address.ts'

assert.equal(truncAddress(''), '')
assert.equal(truncAddress('bc1p'), 'bc1p')
assert.equal(truncAddress('bc1pwxlpge5x8n3z7c0hcusmt2jy5fgq9kly6w0s2rflmakrgjtp0w0qxh66u2'), 'bc1pw…66u2')

console.log('trunc-address: all assertions passed.')
