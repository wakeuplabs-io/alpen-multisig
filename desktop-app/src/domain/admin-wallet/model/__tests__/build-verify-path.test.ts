// build-verify-path helpers — verify-on-device derivation paths (Phase 8, PRD §4.2 / §4.3.4.2).

import assert from 'node:assert/strict'
import { receiveVerifyPathFromAccount } from '../build-verify-path.ts'

// ── receiveVerifyPathFromAccount (BIP-86 / P2TR) ─────────────────────────────
// The receive verify path is built from the connect-returned device-specific account path
// (Trezor 0', Ledger 1' on test nets / 0' on mainnet), not rebuilt from the session network.

assert.equal(receiveVerifyPathFromAccount("m/86'/1'/73'", 0), "m/86'/1'/73'/0/0", 'Ledger regtest receive index 0')
assert.equal(receiveVerifyPathFromAccount("m/86'/1'/73'", 5), "m/86'/1'/73'/0/5", 'Ledger regtest receive index 5')
assert.equal(receiveVerifyPathFromAccount("m/86'/0'/73'", 3), "m/86'/0'/73'/0/3", 'Trezor / mainnet receive index 3')
console.log('receiveVerifyPathFromAccount OK')

console.log('build-verify-path: all assertions passed')
