import assert from 'node:assert/strict'
import { buildSignerSetChange } from '../build-signer-set-change.ts'

const KEY_A = `02${'a'.repeat(64)}`
const KEY_B = `02${'b'.repeat(64)}`
const KEY_C = `02${'c'.repeat(64)}`
const KEY_D = `02${'d'.repeat(64)}`

{
	const result = buildSignerSetChange({
		signers: [KEY_A, KEY_B, KEY_C],
		threshold: 3,
		addKeys: [KEY_D],
		removeKeys: [KEY_C],
		newThreshold: 2,
		isEnacted: false,
	})

	assert.deepEqual(
		result.rows.map(({ pubkey, inBefore, inAfter, isAdded, isRemoved }) => ({
			pubkey,
			inBefore,
			inAfter,
			isAdded,
			isRemoved,
		})),
		[
			{ pubkey: KEY_A, inBefore: true, inAfter: true, isAdded: false, isRemoved: false },
			{ pubkey: KEY_B, inBefore: true, inAfter: true, isAdded: false, isRemoved: false },
			{ pubkey: KEY_C, inBefore: true, inAfter: false, isAdded: false, isRemoved: true },
			{ pubkey: KEY_D, inBefore: false, inAfter: true, isAdded: true, isRemoved: false },
		],
		'current signer order is stable and additions are appended',
	)
	assert.equal(result.thresholdBefore, 3)
	assert.equal(result.thresholdAfter, 2)
}

{
	const result = buildSignerSetChange({
		signers: [KEY_A, KEY_B, KEY_D],
		threshold: 2,
		addKeys: [KEY_D],
		removeKeys: [KEY_C],
		newThreshold: 3,
		isEnacted: true,
	})

	assert.equal(result.thresholdBefore, null, 'an enacted proposal must not invent its historical threshold')
	assert.equal(result.thresholdAfter, 2, 'the enacted after threshold comes from current canonical state')
	assert.deepEqual(
		result.rows.map((row) => row.pubkey),
		[KEY_A, KEY_B, KEY_C, KEY_D],
		'enacted reconstruction keeps reconstructed before rows ahead of additions',
	)
	assert.equal(result.rows.find((row) => row.pubkey === KEY_C)?.isRemoved, true)
	assert.equal(result.rows.find((row) => row.pubkey === KEY_D)?.isAdded, true)
}

{
	const result = buildSignerSetChange({
		signers: [`  0x${KEY_A.toUpperCase()}  `, KEY_B],
		threshold: 2,
		addKeys: [KEY_A],
		removeKeys: [` 0X${KEY_B.toUpperCase()} `],
		newThreshold: 1,
		isEnacted: false,
	})

	assert.equal(result.rows.length, 2, 'canonical comparison must not duplicate differently formatted keys')
	assert.equal(result.rows[0]?.pubkey, KEY_A, 'displayed keys match the canonical payload form')
	assert.equal(result.rows[0]?.isAdded, false, 're-adding the same canonical key is not an addition')
	assert.equal(result.rows[0]?.isRemoved, false)
	assert.equal(result.rows[1]?.isRemoved, true, 'trimmed and 0x-prefixed removals match canonical signers')
}

console.log('buildSignerSetChange: all assertions passed')
