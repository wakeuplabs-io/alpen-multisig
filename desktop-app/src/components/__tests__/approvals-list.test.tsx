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
import { pendingSigners } from '../../lib/pending-signers.ts'
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
assert.ok(
	listSource.includes("isPending ? 'Pending' : 'No signature'"),
	'missing signers must read Pending while pending and No signature otherwise (#540)',
)
assert.ok(listSource.includes('YOU'), 'the connected signer must be marked with a YOU badge')
assert.ok(listSource.includes('No signatures yet.'), 'an empty list must say so instead of rendering nothing')
console.log('ApprovalsList: signed / pending / you / empty states OK')

// ── 3. Pending rows come from the authority signer set ──────────────────────
// The ASM lists Admin IDs and the backend stores keys, so the list compares the two as Admin IDs.
assert.ok(
	listSource.includes('pendingSigners(') && listSource.includes('useAdminIds('),
	'pending rows must be allSigners minus the signatures, compared as Admin IDs',
)
const SIGNER_A = 'bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080'
const SIGNER_B = 'bcrt1qq6hag67dl53wl99vzg42z8eyzfz2xlkvwk6f7m'
assert.deepEqual(pendingSigners([SIGNER_A, SIGNER_B], [SIGNER_A.toUpperCase()]), [SIGNER_B], 'case does not matter')
assert.deepEqual(pendingSigners([SIGNER_A, SIGNER_B], []), [SIGNER_A, SIGNER_B], 'nobody signed: all pending')
assert.deepEqual(
	pendingSigners([SIGNER_A, SIGNER_B], [SIGNER_A, undefined]),
	[],
	'while a signature resolves, no one is listed as pending',
)
assert.deepEqual(pendingSigners([SIGNER_A], ['']), [SIGNER_A], 'a key with no Admin ID matches no signer')
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

// ── 5. The label is keyed off status AND quorum, not off a constant ─────────
// Without this, a caller passing `isPending={true}` would keep #540 fully reproducible on screen
// while the component's own ternary still looked right. Status alone is not enough either: a quorum-full
// proposal stays `pending` until the desktop's PATCH to `approved` lands, and the dashboard already calls
// that state "Quorum reached", so the list must agree with it.
assert.ok(
	detailSource.includes("isPending={displayStatus === 'pending' && !hasQuorum}"),
	'the proposal screen must key the label off the proposal status and quorum (#540)',
)
assert.ok(
	cardSource.includes("isPending={cancelProposal.status === 'pending' && !hasQuorum}"),
	'the cancel card must key the label off the cancellation status and quorum (#540)',
)
console.log('ApprovalsList: missing-signer label keyed off proposal status and quorum OK')

console.log('All ApprovalsList contract tests passed.')
