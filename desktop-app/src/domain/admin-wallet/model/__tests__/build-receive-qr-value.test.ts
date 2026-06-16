// build-receive-qr-value — pure model unit test (tsx + node:assert).
//
// PRD §4.3.4.1: the receive address is shown as text AND QR, and clicking
// either MUST copy *the address*. So the QR payload must be the bare address,
// never a BIP-21 URI, or the copied value would not equal the address.

import assert from 'node:assert/strict'
import { buildReceiveQrValue } from '../build-receive-qr-value.ts'

// Happy path: a typical bech32m receive address is returned unchanged.
const addr = 'bcrt1pwxlpge5x8n3z7c0hcusmt2jy5fgq9kly6w0s2rflmakrgjtp0w0qxh66u2'
assert.equal(buildReceiveQrValue(addr), addr, 'must return the address unchanged')

// Does NOT prepend a URI scheme — copy must yield exactly the address.
assert.ok(!buildReceiveQrValue(addr).startsWith('bitcoin:'), 'must not produce a BIP-21 URI')

// Edge: empty / whitespace-only input → empty string (caller guards rendering).
assert.equal(buildReceiveQrValue(''), '', 'empty input → empty string')
assert.equal(buildReceiveQrValue('   '), '', 'whitespace-only input → empty string')

// Surrounding whitespace is trimmed so the QR/copy value is canonical.
assert.equal(buildReceiveQrValue(`  ${addr}  `), addr, 'trims surrounding whitespace')

console.log('build-receive-qr-value: all assertions passed.')
