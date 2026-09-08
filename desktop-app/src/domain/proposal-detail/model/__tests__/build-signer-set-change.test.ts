// `buildSignerSetChange` — extracted per SDD (V3 Phase 3, §4.10) from the identical Before/After
// row construction `useDecodedProposal` and `useManualProposal` each carried, one with an
// `isEnacted` branch and one without. Pinned here with no DOM and no mocks.

import assert from 'node:assert/strict'
import { buildSignerSetChange } from '../build-signer-set-change.ts'

const KEY_A = `02${'a'.repeat(64)}`
const KEY_B = `02${'b'.repeat(64)}`
const KEY_C = `02${'c'.repeat(64)}`
const KEY_D = `02${'d'.repeat(64)}`

// Claim 1 — the normal (not-yet-enacted) case: `signers`/`threshold` are read as the *before*
// state, `addKeys`/`removeKeys` project the *after* state, and rows are classified accordingly.
{
	const result = buildSignerSetChange({
		signers: [KEY_A, KEY_B, KEY_C],
		threshold: 3,
		addKeys: [KEY_D],
		removeKeys: [KEY_C],
		newThreshold: 2,
		isEnacted: false,
	})

	const byKey = new Map(result.rows.map((row) => [row.pubkey, row]))

	assert.equal(result.rows.length, 4, 'claim 1: A, B, C, D — one row each, no duplicates')
	assert.equal(result.thresholdBefore, 3, 'claim 1: thresholdBefore is the current threshold')
	assert.equal(result.thresholdAfter, 2, 'claim 1: thresholdAfter is the decoded newThreshold')

	// Intact: in both before and after, neither added nor removed.
	assert.deepEqual(
		byKey.get(KEY_A),
		{ pubkey: KEY_A, inBefore: true, inAfter: true, isAdded: false, isRemoved: false },
		'claim 1: A is intact',
	)
	assert.deepEqual(
		byKey.get(KEY_B),
		{ pubkey: KEY_B, inBefore: true, inAfter: true, isAdded: false, isRemoved: false },
		'claim 1: B is intact',
	)
	// Removed: in before, not in after.
	assert.deepEqual(
		byKey.get(KEY_C),
		{ pubkey: KEY_C, inBefore: true, inAfter: false, isAdded: false, isRemoved: true },
		'claim 1: C is removed',
	)
	// Added: not in before, in after.
	assert.deepEqual(
		byKey.get(KEY_D),
		{ pubkey: KEY_D, inBefore: false, inAfter: true, isAdded: true, isRemoved: false },
		'claim 1: D is added',
	)
}

// Claim 2 — the `isEnacted` case: `signers`/`threshold` are the *current* (post-rotation) state,
// and `beforeSigners` is reconstructed by removing the added keys and putting the removed keys
// back — the inverse of claim 1's projection.
{
	const result = buildSignerSetChange({
		signers: [KEY_A, KEY_B, KEY_D],
		threshold: 2,
		addKeys: [KEY_D],
		removeKeys: [KEY_C],
		newThreshold: 2,
		isEnacted: true,
	})

	const byKey = new Map(result.rows.map((row) => [row.pubkey, row]))

	assert.equal(result.thresholdBefore, null, 'claim 2: thresholdBefore is null when enacted')
	assert.equal(result.thresholdAfter, 2, 'claim 2: thresholdAfter is the current threshold, not newThreshold')

	assert.deepEqual(
		byKey.get(KEY_A),
		{ pubkey: KEY_A, inBefore: true, inAfter: true, isAdded: false, isRemoved: false },
		'claim 2: A is intact',
	)
	assert.deepEqual(
		byKey.get(KEY_C),
		{ pubkey: KEY_C, inBefore: true, inAfter: false, isAdded: false, isRemoved: true },
		'claim 2: C reappears in before (it was removed by this rotation)',
	)
	assert.deepEqual(
		byKey.get(KEY_D),
		{ pubkey: KEY_D, inBefore: false, inAfter: true, isAdded: true, isRemoved: false },
		'claim 2: D is added (excluded from the reconstructed before state)',
	)
}

// Claim 3 — keys compare case-insensitively: an add naming the same key with different casing
// than the current signer set is recognised as the same signer, not a spurious add+remove pair.
{
	const upper = KEY_A.toUpperCase()
	const result = buildSignerSetChange({
		signers: [KEY_A, KEY_B],
		threshold: 2,
		addKeys: [upper],
		removeKeys: [],
		newThreshold: 2,
		isEnacted: false,
	})

	assert.equal(result.rows.length, 2, 'claim 3: no spurious extra row for a re-cased key')
	const rowA = result.rows.find((row) => row.pubkey.toLowerCase() === KEY_A.toLowerCase())
	assert.ok(rowA, 'claim 3: the key is present')
	assert.equal(rowA?.isAdded, false, 'claim 3: re-adding the same key (different case) is not a real add')
	assert.equal(rowA?.isRemoved, false, 'claim 3: and not a real removal either')
}

console.log('buildSignerSetChange: all assertions passed')
