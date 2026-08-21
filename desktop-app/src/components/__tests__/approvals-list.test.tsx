// ApprovalsList — the shared "who signed, who is missing, which row is me" list.
//
// SCOPE: pure-logic / source contract only. DOM-rendering assertions would need vitest +
// @testing-library/react (BLOCKED_BY_DEPENDENCY — not installed), so the rows are pinned by
// reading the component source, the same way the other card tests in this repo do.

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// ── 1. Module export resolves ────────────────────────────────────────────────
import { ApprovalsList } from '../approvals-list.tsx'
assert.equal(typeof ApprovalsList, 'function', 'ApprovalsList must be exported')
console.log('ApprovalsList: module export OK')

const __dirname = dirname(fileURLToPath(import.meta.url))
const listSource = readFileSync(join(__dirname, '..', 'approvals-list.tsx'), 'utf8')
const detailSource = readFileSync(
	join(__dirname, '..', '..', 'domain', 'proposal-detail', 'components', 'proposal-detail.tsx'),
	'utf8',
)
const cardSource = readFileSync(
	join(__dirname, '..', '..', 'domain', 'cancel-proposal', 'components', 'cancel-details-card.tsx'),
	'utf8',
)

// ── 2. The three row states the signer reads the list for ───────────────────
assert.ok(listSource.includes('>Signed<'), 'signed rows must be labelled Signed')
assert.ok(listSource.includes('>Pending<'), 'missing signers must be listed as Pending')
assert.ok(listSource.includes('YOU'), 'the connected signer must be marked with a YOU badge')
assert.ok(listSource.includes('No signatures yet.'), 'an empty list must say so instead of rendering nothing')
console.log('ApprovalsList: signed / pending / you / empty states OK')

// ── 3. Pending rows come from the authority signer set ──────────────────────
assert.ok(
	/allSigners\.filter\(/.test(listSource),
	'pending rows must be derived from allSigners minus the collected signatures',
)
assert.ok(
	listSource.includes('.toLowerCase()'),
	'signer comparison must be case-insensitive — pubkey hex casing varies by source',
)
console.log('ApprovalsList: pending rows derived from allSigners OK')

// ── 4. Both screens use this one list ───────────────────────────────────────
// The markup used to live inline in proposal-detail.tsx and the cancel card had no list at all,
// which is how the cancel screen ended up showing strictly less than the proposal screen (#486).
assert.ok(detailSource.includes('<ApprovalsList'), 'the proposal screen must render the shared list')
assert.ok(cardSource.includes('<ApprovalsList'), 'the cancel card must render the shared list')
assert.ok(
	!detailSource.includes('Pending rows — signers not yet in signatures'),
	'proposal-detail.tsx must not keep a duplicate copy of the approvals markup',
)
console.log('ApprovalsList: single implementation shared by both screens OK')

console.log('All ApprovalsList contract tests passed.')
