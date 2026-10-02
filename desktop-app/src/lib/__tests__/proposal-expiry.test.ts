// proposal-expiry — when the pending countdown starts to warn (#551).
//
// The reported bug: the warning fired below a fixed 24 h, and the desktop computed a 1-day lifetime,
// so every proposal read "⚠ Expiring soon" from the second it was created. A signal that is always on
// carries nothing. What is pinned here: the threshold follows from the proposal's real lifetime, so a
// fresh proposal never warns, and the 7-day default keeps its 24 h / 1 h thresholds.

import assert from 'node:assert/strict'
import { expiryUrgency, formatExpiryTimeLeft } from '../proposal-expiry'

const MINUTE = 60 * 1_000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

// ── A fresh proposal never warns, whatever the window ──

assert.equal(expiryUrgency(DAY - MINUTE, DAY), 'none', 'a 1-day proposal one minute old')
assert.equal(expiryUrgency(7 * DAY - MINUTE, 7 * DAY), 'none', 'a 7-day proposal one minute old')

// ── The 7-day default: warn in the last day, urgent in the last hour ──

assert.equal(expiryUrgency(DAY + MINUTE, 7 * DAY), 'none')
assert.equal(expiryUrgency(23 * HOUR, 7 * DAY), 'warning')
assert.equal(expiryUrgency(59 * MINUTE, 7 * DAY), 'urgent')

// ── A short window warns in its last quarter, not from creation ──

assert.equal(expiryUrgency(7 * HOUR, DAY), 'none')
assert.equal(expiryUrgency(5 * HOUR, DAY), 'warning')
assert.equal(expiryUrgency(59 * MINUTE, DAY), 'urgent')

// ── The label ──

assert.equal(formatExpiryTimeLeft(6 * DAY + 23 * HOUR + 59 * MINUTE), 'Expires in 6 d 23 h')
assert.equal(formatExpiryTimeLeft(23 * HOUR + 59 * MINUTE), 'Expires in 23 h 59 m')
assert.equal(formatExpiryTimeLeft(5 * MINUTE), 'Expires in 5 m')
assert.equal(formatExpiryTimeLeft(0), 'Expired')

console.log('proposal-expiry: OK')
