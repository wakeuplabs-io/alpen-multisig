// imported-pair — the import tells the user when the orchestrator ignored its pair (#605).

import assert from 'node:assert/strict'
import { importedPairIgnored, importedTxid } from '../imported-pair'

const REVEAL = 'ab'.repeat(32)
const OTHER = 'cd'.repeat(32)

assert.equal(importedPairIgnored(REVEAL, REVEAL), false, 'the row tracks the imported reveal')
assert.equal(importedPairIgnored(REVEAL, REVEAL.toUpperCase()), false, 'txids compare case-insensitively')
assert.equal(importedPairIgnored(REVEAL, OTHER), true, 'the row tracks another pair')
assert.equal(importedPairIgnored(REVEAL, undefined), true, 'the row tracks no pair')
assert.equal(
	importedPairIgnored(null, OTHER),
	true,
	'an import without a reveal does not move a row that tracks a pair',
)
assert.equal(importedPairIgnored(null, undefined), false, 'an import without a reveal on a row with none is applied')

assert.equal(importedTxid(REVEAL, null, false), REVEAL, 'a txid the row lacks is imported')
assert.equal(importedTxid(OTHER, REVEAL, false), OTHER, 'a different txid is imported')
assert.equal(importedTxid(REVEAL, REVEAL, false), null, 'the same txid with no newer status is left out')
assert.equal(importedTxid(REVEAL, REVEAL, true), REVEAL, 'a newer status for the tracked pair names it')
assert.equal(importedTxid(42, REVEAL, true), null, 'a txid that is not a string is ignored')

console.log('imported-pair: all assertions passed')
