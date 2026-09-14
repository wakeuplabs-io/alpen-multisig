// "View cancel" must open the cancel proposal whatever state the target is in: the cancel
// screen's guards only apply to starting a cancel, never to viewing an existing one.
//
// Source-text assertions, following the project's existing tsx-runner style — React rendering
// tests need vitest + @testing-library/react (BLOCKED_BY_DEPENDENCY — not installed).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const srcDir = join(dirname(fileURLToPath(import.meta.url)), '..', '..')

const cancelScreen = readFileSync(join(srcDir, 'screens', 'cancel-proposal-screen.tsx'), 'utf8')
const detailScreen = readFileSync(join(srcDir, 'screens', 'proposal-detail-screen.tsx'), 'utf8')
const cardSource = readFileSync(
	join(srcDir, 'domain', 'cancel-proposal', 'components', 'cancel-details-card.tsx'),
	'utf8',
)

// The guards only run while no cancel exists yet.
assert.match(cancelScreen, /isInitiateMode = .*cancelProposal === null/, 'initiate mode means no cancel yet')
assert.ok(
	cancelScreen.includes("isInitiateMode && proposal.status !== 'approved'"),
	'cancel-proposal-screen.tsx: the approved guard must only run in initiate mode',
)
assert.ok(
	cancelScreen.includes('isInitiateMode && !canCancelProposal'),
	'cancel-proposal-screen.tsx: the cancelable guard must only run in initiate mode',
)
console.log('cancel-proposal-screen: guards scoped to initiate mode OK')

// An existing cancel is always rendered; the initiate prompt only when there is none.
assert.ok(
	cancelScreen.includes('cancelProposal !== null && ('),
	'cancel-proposal-screen.tsx: an existing cancel must render on its own',
)
assert.ok(
	cancelScreen.includes("cancelProposal === null && proposal.status === 'approved'"),
	'cancel-proposal-screen.tsx: the initiate prompt needs no cancel and an approved target',
)
console.log('cancel-proposal-screen: existing cancel always rendered OK')

// The card and the detail banner read the cancel's own status through the shared predicate.
assert.ok(
	cardSource.includes('isTerminalProposalStatus('),
	'cancel-details-card.tsx: must use the shared terminal predicate',
)
assert.ok(
	detailScreen.includes('isTerminalProposalStatus(proposal.cancelProposal.status)'),
	'proposal-detail-screen.tsx: banner copy must follow the cancel status',
)
assert.ok(detailScreen.includes('View cancel'), 'proposal-detail-screen.tsx: the banner must keep its View cancel link')
console.log('cancel status: shared predicate wired to card and banner OK')
