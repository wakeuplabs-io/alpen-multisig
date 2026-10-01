import assert from 'node:assert/strict'
import { formatInFlightBalanceLine } from '../format-in-flight-balance-line.ts'

assert.equal(formatInFlightBalanceLine(0), null)
assert.equal(formatInFlightBalanceLine(1_234), '1,234 sats in flight')
assert.equal(formatInFlightBalanceLine(NaN), null)

console.log('format-in-flight-balance-line: all assertions passed.')
