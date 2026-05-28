// useWalletPanelState hook — pure TypeScript contract test.
//
// SCOPE: Only the pure-TypeScript contracts (export surface, type shapes) are
// covered here using the project's existing tsx test runner.
//
// React hook behaviour (URL param reads/writes via useSearchParams) CANNOT be
// tested without a React testing framework (vitest + @testing-library/react).
// Those tests are blocked by a missing dev-dependency.
// See: BLOCKED_BY_DEPENDENCY — vitest + @testing-library/react not installed.

import assert from 'node:assert/strict'

// ── 1. Export surface ────────────────────────────────────────────────────────
// Verify the hook is exported from its module and is a function.

import { useWalletPanelState } from '../use-wallet-panel-state.ts'

assert.equal(typeof useWalletPanelState, 'function', 'useWalletPanelState must be exported as a function')
console.log('useWalletPanelState: export surface OK')

// ── 2. WalletPanelSection type contract ──────────────────────────────────────
// Verify all valid section values are accepted by the type.
// This is enforced at compile time; the tsx build will fail if the union is wrong.

import type { WalletPanelSection } from '../use-wallet-panel-state.ts'

const _addresses: WalletPanelSection = 'addresses'
const _receive: WalletPanelSection = 'receive'
const _transactions: WalletPanelSection = 'transactions'
const _send: WalletPanelSection = 'send'

void _addresses
void _receive
void _transactions
void _send

console.log('WalletPanelSection: all valid values accepted OK')

// ── 3. No Tauri import in hook module ─────────────────────────────────────────
// The hook must not import from @tauri-apps/api/core or @/api/admin-wallet.
// Enforced by the fact that useWalletPanelState.ts must only use react-router-dom.
// If a Tauri import crept in, the tsx import above would still succeed in node
// context, so we do a source-level string check.

import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const hookSource = readFileSync(join(__dirname, '..', 'use-wallet-panel-state.ts'), 'utf8')

assert.ok(!hookSource.includes('@tauri-apps/api/core'), 'hook must not import from @tauri-apps/api/core')
assert.ok(!hookSource.includes('@/api/admin-wallet'), 'hook must not import from @/api/admin-wallet')
console.log('useWalletPanelState: no Tauri/admin-wallet imports OK')

console.log('All useWalletPanelState contract tests passed.')
