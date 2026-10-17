// imported-pair — the import tells the user what the orchestrator did with its pair (#605).

import assert from 'node:assert/strict'
import { IMPORT_NOT_RECORDED_NOTICE, IMPORTED_PAIR_LISTED_NOTICE, importNotice, importedTxid } from '../imported-pair'

const REVEAL = 'ab'.repeat(32)
const OTHER = 'cd'.repeat(32)

assert.equal(importNotice(REVEAL, REVEAL), null, 'the row tracks the imported reveal')
assert.equal(importNotice(REVEAL, REVEAL.toUpperCase()), null, 'txids compare case-insensitively')
assert.equal(importNotice(REVEAL, OTHER), IMPORTED_PAIR_LISTED_NOTICE, 'the row tracks another pair: listed beside it')
assert.equal(importNotice(null, OTHER), IMPORT_NOT_RECORDED_NOTICE, 'no reveal does not move a row that tracks one')
assert.equal(importNotice(null, undefined), null, 'no reveal on a row with none is applied')
assert.equal(importedTxid(REVEAL, null, false), REVEAL, 'a txid the row lacks is imported')
assert.equal(importedTxid(OTHER, REVEAL, false), OTHER, 'a different txid is imported')
assert.equal(importedTxid(REVEAL, REVEAL, false), null, 'the same txid with no newer status is left out')
assert.equal(importedTxid(REVEAL, REVEAL, true), REVEAL, 'a newer status for the tracked pair names it')
assert.equal(importedTxid(42, REVEAL, true), null, 'a txid that is not a string is ignored')

console.log('imported-pair: all assertions passed')
