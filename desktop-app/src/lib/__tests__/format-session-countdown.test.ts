// format-session-countdown — the time left on the session chip (#582).
//
// The session now lasts 24 hours from sign-in. The old `mm:ss` label had no hours field, so a fresh
// session would have read `1440:00`. What is pinned here: the label is always `HH:MM:SS`, it counts
// down whole seconds, and no session reads as a placeholder of the same shape.

import assert from 'node:assert/strict'
import { formatSessionCountdown } from '../format-session-countdown'

const SECOND = 1_000
const MINUTE = 60 * SECOND
const HOUR = 60 * MINUTE

// ── Always hours, minutes and seconds, from a full day down to zero ──

assert.equal(formatSessionCountdown(24 * HOUR), '24:00:00')
assert.equal(formatSessionCountdown(24 * HOUR - SECOND), '23:59:59')
assert.equal(formatSessionCountdown(HOUR), '01:00:00')
assert.equal(formatSessionCountdown(HOUR - SECOND), '00:59:59')
assert.equal(formatSessionCountdown(5 * MINUTE - SECOND), '00:04:59')
assert.equal(formatSessionCountdown(0), '00:00:00')

// ── A partial second never rounds up: the label must not promise time that is not there ──

assert.equal(formatSessionCountdown(5 * MINUTE - 1), '00:04:59')
assert.equal(formatSessionCountdown(999), '00:00:00')

// ── No session reads as a placeholder of the same width ──

assert.equal(formatSessionCountdown(null), '--:--:--')

console.log('format-session-countdown: all assertions passed')
