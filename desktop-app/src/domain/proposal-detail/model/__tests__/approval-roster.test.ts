import assert from 'node:assert/strict'
import { buildSignerSetChange } from '../../../signer-set-change/model/build-signer-set-change.ts'
import { approvalRoster } from '../approval-roster.ts'

const SIGNER_A = '02' + 'a'.repeat(64)
const SIGNER_B = '02' + 'b'.repeat(64)
const SIGNER_C = '03' + 'c'.repeat(64)
const SIGNER_D = '03' + 'd'.repeat(64)
const SIGNER_E = '02' + 'e'.repeat(64)

// ── REGRESSION #603: an enacted removal keeps the removed signer in the roster ─
// The live config is already post-rotation, so D is gone from it. D was a member when the
// proposal was signed and must still be listed (as "No signature" if they never signed).
{
	const change = buildSignerSetChange({
		signers: [SIGNER_A, SIGNER_B, SIGNER_C],
		threshold: 2,
		addKeys: [],
		removeKeys: [SIGNER_D],
		newThreshold: 2,
		isEnacted: true,
	})
	const roster = approvalRoster([SIGNER_A, SIGNER_B, SIGNER_C], change)
	assert.deepEqual([...roster].sort(), [SIGNER_A, SIGNER_B, SIGNER_C, SIGNER_D].sort())
}

// ── An enacted addition leaves out the signer who joined through it ──────────
// E only became a member once this proposal was enacted, so E could not have approved it.
{
	const change = buildSignerSetChange({
		signers: [SIGNER_A, SIGNER_B, SIGNER_E],
		threshold: 2,
		addKeys: [SIGNER_E],
		removeKeys: [],
		newThreshold: 2,
		isEnacted: true,
	})
	const roster = approvalRoster([SIGNER_A, SIGNER_B, SIGNER_E], change)
	assert.deepEqual([...roster].sort(), [SIGNER_A, SIGNER_B].sort())
}

// ── No signer-set change: the current config is the roster ───────────────────
{
	const current = [SIGNER_A, SIGNER_B, SIGNER_C]
	assert.deepEqual(approvalRoster(current, null), current)
}

// ── Pubkeys from the action are normalized like the rest of the roster ───────
{
	const change = buildSignerSetChange({
		signers: [SIGNER_A, SIGNER_B],
		threshold: 2,
		addKeys: [],
		removeKeys: [`0x${SIGNER_D.toUpperCase()}`],
		newThreshold: 2,
		isEnacted: true,
	})
	const roster = approvalRoster([SIGNER_A, SIGNER_B], change)
	assert.ok(roster.includes(SIGNER_D))
	assert.equal(roster.length, 3)
}

console.log('approval-roster: all assertions passed')
