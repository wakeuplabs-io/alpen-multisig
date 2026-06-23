// build-verify-path helpers — verify-on-device derivation paths (Phase 8, PRD §4.2 / §4.3.4.2).

import assert from 'node:assert/strict'
import { buildReceiveVerifyPath, coinType } from '../build-verify-path.ts'

// ── coinType ─────────────────────────────────────────────────────────────────

assert.equal(coinType('bitcoin'), 0, 'mainnet uses coin type 0')
assert.equal(coinType('mainnet'), 0, 'mainnet alias uses coin type 0')
assert.equal(coinType('testnet'), 1, 'testnet uses coin type 1')
assert.equal(coinType('regtest'), 1, 'regtest uses coin type 1')
assert.equal(coinType('signet'), 1, 'signet uses coin type 1')
console.log('coinType OK')

// ── buildReceiveVerifyPath (BIP-86 / P2TR) ───────────────────────────────────

assert.equal(buildReceiveVerifyPath('regtest', 0), "m/86'/1'/73'/0/0", 'regtest receive index 0')
assert.equal(buildReceiveVerifyPath('regtest', 5), "m/86'/1'/73'/0/5", 'regtest receive index 5')
assert.equal(buildReceiveVerifyPath('bitcoin', 3), "m/86'/0'/73'/0/3", 'mainnet receive index 3')
console.log('buildReceiveVerifyPath OK')

console.log('build-verify-path: all assertions passed')
