// useAdminWalletCapability — signerKind and canSignReason surfaced from capability hook.
//
// SCOPE: Export surface and source-code composition contract for the evolved
// admin_wallet_can_sign DTO: { canSign, signerKind, reason? }.
// Legacy bare-bool responses must gracefully degrade to signerKind: 'none'.

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const __dirname = dirname(fileURLToPath(import.meta.url))

// ── 1. Export surface ────────────────────────────────────────────────────────

import { useAdminWalletCapability } from '../use-admin-wallet-capability.ts'

assert.equal(typeof useAdminWalletCapability, 'function', 'useAdminWalletCapability must be exported as a function')
console.log('useAdminWalletCapability: export surface OK')

// ── 2. Hook source references getAdminWalletCanSign ──────────────────────────

const hookSource = readFileSync(join(__dirname, '..', 'use-admin-wallet-capability.ts'), 'utf8')

assert.ok(hookSource.includes('getAdminWalletCanSign'), 'hook must call getAdminWalletCanSign')
console.log('useAdminWalletCapability: calls getAdminWalletCanSign OK')

// ── 3. Hook returns signerKind ───────────────────────────────────────────────

assert.ok(hookSource.includes('signerKind'), 'hook must return signerKind from capability DTO')
console.log('useAdminWalletCapability: returns signerKind OK')

// ── 4. Hook returns canSignReason ────────────────────────────────────────────

assert.ok(hookSource.includes('canSignReason'), 'hook must return canSignReason from capability DTO')
console.log('useAdminWalletCapability: returns canSignReason OK')

// ── 5. Zod schema for signerKind capability DTO exists in ipc-schemas ────────

const ipcSchemasSource = readFileSync(join(__dirname, '..', '..', '..', '..', 'api', 'ipc-schemas.ts'), 'utf8')

assert.ok(
	ipcSchemasSource.includes('signerKindSchema') || ipcSchemasSource.includes('signerKind'),
	'ipc-schemas must define signerKind zod schema',
)
console.log('ipc-schemas: signerKind schema OK')

// ── 6. API wrapper returns typed capability DTO ─────────────────────────────

const adminWalletSource = readFileSync(join(__dirname, '..', '..', '..', '..', 'api', 'admin-wallet.ts'), 'utf8')

assert.ok(
	adminWalletSource.includes('signerKind') || adminWalletSource.includes('SignerKind'),
	'admin-wallet.ts must reference signerKind type',
)
console.log('admin-wallet.ts: signerKind type reference OK')

console.log('All useAdminWalletCapability signerKind contract tests passed.')
