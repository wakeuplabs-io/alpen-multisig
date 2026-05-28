import assert from 'node:assert/strict'
import { relativeTime } from '../relative-time.ts'

const now = new Date('2024-01-01T12:00:00Z')

assert.equal(relativeTime(new Date(now.getTime() - 30_000).toISOString(), now), '30s ago')
assert.equal(relativeTime(new Date(now.getTime() - 5 * 60_000).toISOString(), now), '5m ago')
assert.equal(relativeTime(new Date(now.getTime() - 2 * 3_600_000).toISOString(), now), '2h ago')
assert.equal(relativeTime(new Date(now.getTime() + 1000).toISOString(), now), 'just now')
assert.equal(relativeTime('not-a-date', now), '—')

console.log('relative-time: all assertions passed.')
