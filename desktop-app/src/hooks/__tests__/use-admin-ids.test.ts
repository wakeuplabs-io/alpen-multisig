// useAdminIds — which signer keys a render still has to resolve, and what a batch answer stores.
//
// The hook itself needs a React renderer (not installed); its decisions live in these two pure functions.

import assert from 'node:assert/strict'
import { entriesForBatch, missingPubkeys } from '../use-admin-ids.ts'

const KEY = '0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798'
const OTHER = '02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5'

// ── missingPubkeys ──────────────────────────────────────────────────────────
assert.deepEqual(missingPubkeys([KEY, OTHER], new Map()), [KEY, OTHER].sort(), 'nothing known: every key is missing')
assert.deepEqual(
	missingPubkeys([`0x${KEY.toUpperCase()}`, KEY], new Map()),
	[KEY],
	'keys are normalized and deduplicated before they are requested',
)
assert.deepEqual(
	missingPubkeys([KEY, OTHER], new Map([[KEY, 'bcrt1q…']])),
	[OTHER],
	'a resolved key is not asked again',
)
assert.deepEqual(missingPubkeys([KEY], new Map([[KEY, '']])), [], 'a key known to have no Admin ID is not asked again')
assert.equal(
	missingPubkeys([OTHER, KEY], new Map()).join(','),
	missingPubkeys([KEY, OTHER], new Map()).join(','),
	'the request key is stable across caller order, so the effect does not re-run for a reordered list',
)
console.log('useAdminIds: missingPubkeys OK')

// ── entriesForBatch ─────────────────────────────────────────────────────────
assert.deepEqual(entriesForBatch([KEY, OTHER], { [KEY]: 'bcrt1qw508' }), [
	[KEY, 'bcrt1qw508'],
	[OTHER, ''],
])
assert.deepEqual(entriesForBatch([KEY], null), [[KEY, '']], 'a failed call resolves every key to no Admin ID')
console.log('useAdminIds: entriesForBatch OK')
