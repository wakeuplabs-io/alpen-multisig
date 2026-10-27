import assert from 'node:assert/strict'
import { buildSignerSetChange } from '../build-signer-set-change.ts'

// Regtest Admin IDs, as the ASM lists signers and decoded actions name them.
const adminId = (c: string) => `bcrt1q${c.repeat(38)}`
const KEY_A = adminId('q')
const KEY_B = adminId('p')
const KEY_C = adminId('z')
const KEY_D = adminId('r')

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
		result.rows.map(({ adminId, inBefore, inAfter, isAdded, isRemoved }) => ({
			adminId,
			inBefore,
			inAfter,
			isAdded,
			isRemoved,
		})),
		[
			{ adminId: KEY_A, inBefore: true, inAfter: true, isAdded: false, isRemoved: false },
			{ adminId: KEY_B, inBefore: true, inAfter: true, isAdded: false, isRemoved: false },
			{ adminId: KEY_C, inBefore: true, inAfter: false, isAdded: false, isRemoved: true },
			{ adminId: KEY_D, inBefore: false, inAfter: true, isAdded: true, isRemoved: false },
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
		result.rows.map((row) => row.adminId),
		[KEY_A, KEY_B, KEY_C, KEY_D],
		'enacted reconstruction keeps reconstructed before rows ahead of additions',
	)
	assert.equal(result.rows.find((row) => row.adminId === KEY_C)?.isRemoved, true)
	assert.equal(result.rows.find((row) => row.adminId === KEY_D)?.isAdded, true)
}

{
	const result = buildSignerSetChange({
		signers: [`  ${KEY_A.toUpperCase()}  `, KEY_B],
		threshold: 2,
		addKeys: [KEY_A],
		removeKeys: [` ${KEY_B.toUpperCase()} `],
		newThreshold: 1,
		isEnacted: false,
	})

	assert.equal(result.rows.length, 2, 'canonical comparison must not duplicate differently formatted Admin IDs')
	assert.equal(result.rows[0]?.adminId, KEY_A, 'displayed Admin IDs are the canonical lowercase form')
	assert.equal(result.rows[0]?.isAdded, false, 're-adding the same Admin ID is not an addition')
	assert.equal(result.rows[0]?.isRemoved, false)
	assert.equal(result.rows[1]?.isRemoved, true, 'trimmed, uppercased removals match canonical signers')
}

console.log('buildSignerSetChange: all assertions passed')
