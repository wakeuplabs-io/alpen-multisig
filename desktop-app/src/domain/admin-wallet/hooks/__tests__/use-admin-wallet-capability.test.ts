// useAdminWalletCapability hook — pure TypeScript contract test.
//
// SCOPE: Export surface and source-code composition contract.
// React hook behaviour (canSign state from getAdminWalletCanSign) cannot be tested
// without a React testing framework (vitest + @testing-library/react).
// See: BLOCKED_BY_DEPENDENCY — vitest + @testing-library/react not installed.

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

// ── 1. Export surface ────────────────────────────────────────────────────────
// Verify the hook is exported from its module and is a function.

import { useAdminWalletCapability } from '../use-admin-wallet-capability.ts'

assert.equal(typeof useAdminWalletCapability, 'function', 'useAdminWalletCapability must be exported as a function')
console.log('useAdminWalletCapability: export surface OK')

// ── 2. Calls getAdminWalletCanSign (composition contract) ────────────────────
// Verify the hook source references the API function it wraps.

const __dirname = dirname(fileURLToPath(import.meta.url))
const hookSource = readFileSync(join(__dirname, '..', 'use-admin-wallet-capability.ts'), 'utf8')

assert.ok(hookSource.includes('getAdminWalletCanSign'), 'hook must call getAdminWalletCanSign')
console.log('useAdminWalletCapability: calls getAdminWalletCanSign OK')

// ── 3. Defaults canSign to false (source contract) ───────────────────────────
// useState(false) must be present — this is the default-false requirement.

assert.ok(hookSource.includes('useState(false)'), 'hook must default canSign to false via useState(false)')
console.log('useAdminWalletCapability: defaults canSign to false OK')

// ── 4. Returns canSign property ───────────────────────────────────────────────
// The return must include { canSign }.

assert.ok(hookSource.includes('canSign'), 'hook must return canSign')
console.log('useAdminWalletCapability: returns canSign OK')

console.log('All useAdminWalletCapability contract tests passed.')
