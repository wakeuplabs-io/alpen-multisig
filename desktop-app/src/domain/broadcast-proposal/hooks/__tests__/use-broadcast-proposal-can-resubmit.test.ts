// useBroadcastProposal — canResubmit recovery-driven contract test.
//
// SCOPE: Verifies that canResubmit is derived from error.recovery === 'resubmit-reveal',
// NEVER from Boolean(error). Pre-broadcast errors must yield canResubmit=false.
//
// React hook behaviour cannot be tested without a React testing framework
// (vitest + @testing-library/react). This is a source-code composition contract.

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const hookSource = readFileSync(join(__dirname, '..', 'use-broadcast-proposal.ts'), 'utf8')

// ── 1. canResubmit is recovery-driven, never Boolean(error) ──────────────────
// The hook must compute canResubmit from error.recovery === 'resubmit-reveal'.
// It must NOT compute it as Boolean(error) or !!error or error != null.

assert.ok(
	hookSource.includes("recovery === 'resubmit-reveal'") || hookSource.includes('recovery === "resubmit-reveal"'),
	'canResubmit must be derived from error.recovery === "resubmit-reveal"',
)

// Must NOT use Boolean(error) pattern for canResubmit
const booleanErrorPattern =
	/canResubmit.*Boolean\(error\)|canResubmit.*!!error|canResubmit.*error\s*!=\s*null|canResubmit.*error\s*\?\s*true\s*:\s*false/
assert.ok(!booleanErrorPattern.test(hookSource), 'canResubmit must NEVER be Boolean(error) — must be recovery-driven')

console.log('useBroadcastProposal: canResubmit is recovery-driven OK')

// ── 2. Hook imports deriveBroadcastError to parse structured errors ──────────
// The hook must import and use deriveBroadcastError to get the recovery field.

assert.ok(
	hookSource.includes('deriveBroadcastError'),
	'hook must import and use deriveBroadcastError to parse structured errors',
)

console.log('useBroadcastProposal: imports deriveBroadcastError OK')

// ── 3. Return type includes canResubmit ──────────────────────────────────────

assert.ok(hookSource.includes('canResubmit'), 'hook return type must include canResubmit')

console.log('useBroadcastProposal: returns canResubmit OK')

console.log('use-broadcast-proposal-can-resubmit: all contract tests passed.')
