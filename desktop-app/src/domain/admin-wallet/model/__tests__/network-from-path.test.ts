// network-from-path — infers the verify-on-device network token from a path's coin type.

import assert from 'node:assert/strict'
import { networkFromPath } from '../network-from-path.ts'

// Coin type 0' (Trezor Admin ID, mainnet) → bitcoin so the device coin is "Bitcoin".
assert.equal(networkFromPath("m/84'/0'/73'/0/0"), 'bitcoin', "coin type 0' → bitcoin")
assert.equal(networkFromPath('m/84h/0h/73h/0/0'), 'bitcoin', 'coin type 0h → bitcoin')
assert.equal(networkFromPath("m/86'/0'/73'/0/3"), 'bitcoin', "taproot coin type 0' → bitcoin")

// Coin type 1' (Ledger Admin ID on test nets) → regtest so the device coin is "Testnet".
assert.equal(networkFromPath("m/84'/1'/73'/0/0"), 'regtest', "coin type 1' → regtest")
assert.equal(networkFromPath('m/84h/1h/73h/0/0'), 'regtest', 'coin type 1h → regtest')
assert.equal(networkFromPath("m/86'/1'/73'/0/5"), 'regtest', "taproot coin type 1' → regtest")

console.log('network-from-path: all assertions passed')
