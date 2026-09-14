// SignProposalView — the review screen must say "signer", never "member"

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// ── 1. Module export resolves ────────────────────────────────────────────────
import { SignProposalView } from '../sign-proposal-view.tsx'
assert.equal(typeof SignProposalView, 'function', 'SignProposalView must be exported')
console.log('SignProposalView: module export OK')

const __dirname = dirname(fileURLToPath(import.meta.url))
const viewSource = readFileSync(join(__dirname, '..', 'sign-proposal-view.tsx'), 'utf8')

// ── 2. Every signer-set label says "signer", not "member"  ────────────
assert.ok(viewSource.includes('Signers to add'), 'added keys must be labelled Signers to add')
assert.ok(viewSource.includes('Signers to remove'), 'removed keys must be labelled Signers to remove')
assert.ok(
	viewSource.includes('no signers added or removed'),
	'the threshold-only fallback must say signers, not members',
)
assert.ok(
	!/\bmembers?\b/i.test(viewSource),
	'the review screen must not say "member" anywhere — one term, "signer", for one concept',
)
console.log('SignProposalView: signer-set labels say signer, not member OK')

console.log('All SignProposalView contract tests passed.')
