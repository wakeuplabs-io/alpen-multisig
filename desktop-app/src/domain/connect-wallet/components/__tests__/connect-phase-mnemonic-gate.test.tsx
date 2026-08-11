// The seed-words box belongs to the mnemonic method only (#461).
//
// Selecting Trezor used to leave the mnemonic textarea sitting right underneath the vendor
// chips, on the very screen where the signer decides which keys will sign. A hardware device
// *replaces* mnemonic entry; showing both reads as "you may also need to type your seed here".
// The affordance is dev/QA-only (gated by `mnemonicEnabled`, which mirrors the backend
// `dev_mnemonic_signing_ipc_enabled` gate), and QA and demo passes run on exactly those builds.
//
// This is a source-contract test rather than a rendering one: the project has no vitest /
// @testing-library / jsdom (BLOCKED_BY_DEPENDENCY), and the tsx + node:assert runner used here
// cannot mount React. What it does guard is the thing that would regress — the JSX condition in
// front of each control — by reading the guard line the way a reviewer would.

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// ── 1. Module export resolves ────────────────────────────────────────────────
import { ConnectPhase } from '../connect-phase.tsx'
assert.equal(typeof ConnectPhase, 'function', 'ConnectPhase must be exported')

const __dirname = dirname(fileURLToPath(import.meta.url))
const source = readFileSync(join(__dirname, '..', 'connect-phase.tsx'), 'utf8')
const lines = source.split('\n')

/**
 * The JSX condition standing in front of the element carrying `testid`: walking up from the
 * element, the first line that opens a JSX expression block (`{…`). Reading the guard rather
 * than matching a literal line keeps the assertions alive across reformatting.
 */
function guardFor(testid: string): string {
	const at = lines.findIndex((line) => line.includes(`data-testid="${testid}"`))
	assert.notEqual(at, -1, `expected connect-phase to render an element with data-testid="${testid}"`)
	for (let i = at; i >= 0; i--) {
		if (lines[i].trimStart().startsWith('{')) return lines[i].trim()
	}
	assert.fail(`no JSX condition found in front of data-testid="${testid}"`)
}

// ── 2. The textarea is gated on the mnemonic method being the selected one ───
const textareaGuard = guardFor('e2e-connect-mnemonic-textarea')
assert.ok(textareaGuard.includes('mnemonicEnabled'), 'the mnemonic textarea must stay behind the dev capability gate')
assert.ok(
	/walletVendor === 'mnemonic'/.test(textareaGuard),
	'the mnemonic textarea must render only while mnemonic is the selected method — with Trezor or ' +
		`Ledger selected there is no seed-words box (guard was: ${textareaGuard})`,
)

// One textarea, one guard: a second copy elsewhere in the tree would slip past the check above.
assert.equal(
	source.split('data-testid="e2e-connect-mnemonic-textarea"').length - 1,
	1,
	'connect-phase must render exactly one mnemonic textarea',
)

// ── 3. The Mnemonic chip stays reachable from a device selection ─────────────
// It is how the method gets selected in the first place; gating it on the method would make the
// textarea unreachable.
// `data-testid="e2e-connect-mnemonic"` — closing quote included, so this does not match the
// textarea's `e2e-connect-mnemonic-textarea`.
const chipGuard = guardFor('e2e-connect-mnemonic')
assert.ok(chipGuard.includes('mnemonicEnabled'), 'the Mnemonic chip must stay behind the dev capability gate')
assert.ok(
	!/walletVendor === 'mnemonic'/.test(chipGuard),
	`the Mnemonic chip must not require mnemonic to already be selected (guard was: ${chipGuard})`,
)

console.log('ConnectPhase: mnemonic textarea gated on the selected method OK')
