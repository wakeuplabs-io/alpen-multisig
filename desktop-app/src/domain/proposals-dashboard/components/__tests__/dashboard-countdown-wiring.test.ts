// The dashboard card must show the same activation countdown the detail and cancel screens
// already show, driven by the same live block height and the same shared predicate.
//
// Phase 6 (`security-council-defcon-3-phase-6.md` §3.3, §6): before this, a queued proposal's
// dashboard card said only "Refresh to check whether the ASM has applied it." — no activation
// block, no current block, no countdown. `showsActivationCountdown` already answered correctly for
// a queued Defcon 3; the dashboard just never asked it.
//
// Source-text assertions, following the project's existing tsx-runner style — React rendering
// tests need vitest + @testing-library/react (BLOCKED_BY_DEPENDENCY — not installed). Written
// against files, not components: `ProposalCard` and `ProposalsDashboard` live in one file, and no
// test here parses TSX.

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const componentsDir = join(dirname(fileURLToPath(import.meta.url)), '..')
const screensDir = join(componentsDir, '..', '..', '..', 'screens')

const dashboardComponent = readFileSync(join(componentsDir, 'proposals-dashboard.tsx'), 'utf8')
const dashboardScreen = readFileSync(join(screensDir, 'proposals-dashboard-screen.tsx'), 'utf8')

// The card must not re-derive the countdown condition — the exact defect §3.2 removed from the
// cancel screen.
assert.ok(
	dashboardComponent.includes('showsActivationCountdown('),
	'proposals-dashboard.tsx: must ask the shared predicate, not re-derive the condition',
)

assert.ok(
	dashboardComponent.includes('<ActivationCountdown'),
	'proposals-dashboard.tsx: must render the shared ActivationCountdown component',
)
assert.ok(
	dashboardComponent.includes('currentHeight={currentBlockHeight}'),
	'proposals-dashboard.tsx: must feed the live block height into the countdown',
)

// The countdown displaces the refresh line, so it must not take its place while saying nothing:
// with no tip, `ActivationCountdown` renders the activation block alone — neither how far away it
// is nor what to do next. A null height is not the same case, and is guarded separately.
// Asserted as its own term rather than as part of the whole condition: Prettier wraps a
// three-clause JSX condition across lines, so pinning the joined expression pins the formatter.
assert.ok(
	dashboardComponent.includes('currentBlockHeight !== null'),
	'proposals-dashboard.tsx: an unknown tip must keep the refresh line, not show a countdown to nothing',
)

// One poller for the screen, not one per row: `useBlockHeight` belongs in the screen, not the card.
// Matched as a call and not as a bare identifier, so writing down *why* it is not called here does
// not turn this red.
assert.ok(
	!dashboardComponent.includes('useBlockHeight('),
	'proposals-dashboard.tsx: must not call useBlockHeight itself — one poller per screen, not per row',
)
assert.ok(
	dashboardScreen.includes('useBlockHeight('),
	'proposals-dashboard-screen.tsx: must call useBlockHeight and pass it down',
)

console.log('proposals dashboard: activation countdown wiring OK')
