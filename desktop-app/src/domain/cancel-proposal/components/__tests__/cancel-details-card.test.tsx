// CancelDetailsCard — the card must answer "who signed, am I one of them, who is missing", and
// show the cancellation's own identifying details (#486).
//
// SCOPE: pure-logic / source contract only, like the other card tests here — DOM rendering needs
// vitest + @testing-library/react (BLOCKED_BY_DEPENDENCY — not installed).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// ── 1. Module export resolves ────────────────────────────────────────────────
import { CancelDetailsCard } from '../cancel-details-card.tsx'
assert.equal(typeof CancelDetailsCard, 'function', 'CancelDetailsCard must be exported')
console.log('CancelDetailsCard: module export OK')

const __dirname = dirname(fileURLToPath(import.meta.url))
const cardSource = readFileSync(join(__dirname, '..', 'cancel-details-card.tsx'), 'utf8')
const screenSource = readFileSync(
	join(__dirname, '..', '..', '..', '..', 'screens', 'cancel-proposal-screen.tsx'),
	'utf8',
)

// ── 2. The approvals list is rendered, not a bare counter ───────────────────
assert.ok(cardSource.includes('<ApprovalsList'), 'the card must list the signers of the cancellation')
assert.ok(cardSource.includes('title="Cancel approvals"'), 'the list must be labelled as the cancellation’s own')
console.log('CancelDetailsCard: approvals list rendered OK')

// ── 3. The three spec details from docs/specs/cancel-approved-proposal.md ───
assert.ok(cardSource.includes('Cancel #'), 'the cancellation’s sequence number must be shown')
assert.ok(cardSource.includes('Cancels '), 'the update being cancelled must be identified')
assert.ok(cardSource.includes('Cancel payload'), 'the reviewable payload must be shown')
assert.ok(
	cardSource.includes('<CopyButton text={cancelActionHex}'),
	'the payload must be copyable — a signer reviews it outside the app before approving',
)
console.log('CancelDetailsCard: seq no + target + payload with copy OK')

// ── 4. Own participation survives quorum ────────────────────────────────────
// The note used to be the `else` branch of the quorum ternary, so it disappeared exactly when the
// signer was about to broadcast an irreversible action.
const noteIndex = cardSource.indexOf('You have signed this cancellation.')
assert.ok(noteIndex > 0, 'the card must state that the connected signer has signed')
const noteBlock = cardSource.slice(cardSource.lastIndexOf('{alreadySigned &&', noteIndex), noteIndex)
assert.ok(
	noteBlock.startsWith('{alreadySigned &&'),
	'the note must be gated on alreadySigned alone, never nested under hasQuorum',
)
console.log('CancelDetailsCard: signed note independent of quorum OK')

// ── 5. Display props are required, so an unwired screen fails tsc ───────────
// Optional props are how #484 happened: a screen forgot to pass them and the card silently
// rendered a degraded state forever.
const propsBlock = cardSource.slice(cardSource.indexOf('type Props = {'), cardSource.indexOf('export function'))
for (const prop of [
	'cancelSeqNo',
	'cancelActionHex',
	'isLoadingDetails',
	'targetActionId',
	'targetUpdateId',
	'allSigners',
	'signerPubkey',
]) {
	assert.ok(new RegExp(`\\n\\t${prop}: `).test(propsBlock), `${prop} must stay a required prop`)
}
console.log('CancelDetailsCard: display props required OK')

// ── 6. The screen wires them from data it already holds ─────────────────────
for (const wiring of [
	'cancelSeqNo={cancelDetails.cancelProposal?.seqNo ?? null}',
	'cancelActionHex={cancelDetails.cancelProposal?.actionHex ?? null}',
	'allSigners={decodedData.allSigners}',
	'targetUpdateId={proposal.updateIdInQueue}',
]) {
	assert.ok(screenSource.includes(wiring), `cancel-proposal-screen must wire ${wiring}`)
}
assert.ok(
	screenSource.includes('useSignerPubkey('),
	'the screen must fall back to the session pubkey — router state is lost on refresh',
)
console.log('CancelDetailsCard: screen wiring OK')

console.log('All CancelDetailsCard contract tests passed.')
