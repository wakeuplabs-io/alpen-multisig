// useSendForm — pure TypeScript contract tests (Phase 6 P6.2, PRD §4.3.5.1).
//
// SCOPE: export surface, type shapes, and source-level composition contracts
// using the project's tsx test runner (React hook behaviour needs vitest +
// @testing-library/react, which are not installed — same constraint as
// use-send.test.ts).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

// ── 1. Export surface ────────────────────────────────────────────────────────

import { useSendForm, VALIDATE_DEBOUNCE_MS } from '../use-send-form.ts'
import type { DestinationState } from '../use-send-form.ts'

assert.equal(typeof useSendForm, 'function', 'useSendForm must be exported as a function')
assert.equal(VALIDATE_DEBOUNCE_MS, 300, 'debounce window is 300 ms per spec §6.3')
console.log('useSendForm: export surface OK')

// ── 2. DestinationState type contract ────────────────────────────────────────

const _empty: DestinationState = { status: 'empty' }
const _validating: DestinationState = { status: 'validating', address: 'a' }
const _valid: DestinationState = { status: 'valid', address: 'a' }
const _invalidAddr: DestinationState = {
	status: 'invalid',
	address: 'a',
	reason: 'invalid-address',
	expectedNetwork: 'regtest',
}
const _wrongNet: DestinationState = {
	status: 'invalid',
	address: 'a',
	reason: 'wrong-network',
	expectedNetwork: 'regtest',
}
const _unavailable: DestinationState = { status: 'unavailable', address: 'a' }
void [_empty, _validating, _valid, _invalidAddr, _wrongNet, _unavailable]
console.log('DestinationState: all variants accepted OK')

// ── 3. Composition contracts ─────────────────────────────────────────────────

const __dirname = dirname(fileURLToPath(import.meta.url))
const source = readFileSync(join(__dirname, '..', 'use-send-form.ts'), 'utf8')

assert.ok(!source.includes('@tauri-apps/api/core'), 'hook must not import Tauri directly')
assert.ok(source.includes('validateSendAddress'), 'hook must validate through the typed API adapter')
assert.ok(source.includes('clearTimeout'), 'debounce timer must be cleared on keystroke/unmount')
assert.ok(source.includes('requestSeq'), 'stale in-flight responses must be discarded via a sequence guard')
assert.ok(
	source.includes("setDestination({ status: 'unavailable'"),
	'a failing validation IPC must block Confirm via the unavailable state — never silently valid',
)
console.log('useSendForm: composition contracts OK')

console.log('use-send-form.test.ts PASS')
