import assert from 'node:assert/strict'
import { formatSyncProgressLabel } from '../format-sync-progress-label.ts'
import { makeSyncStatus } from '../__fixtures__/make-sync-status.ts'

// Happy path: thousands grouping + template
assert.equal(
	formatSyncProgressLabel({ processedBlocks: 234, totalBlocks: 1277, percent: 18 }),
	'Syncing 234 / 1,277 blocks (18%)',
)

// Full progress
assert.equal(
	formatSyncProgressLabel({ processedBlocks: 1277, totalBlocks: 1277, percent: 100 }),
	'Syncing 1,277 / 1,277 blocks (100%)',
)

// Edge: zero total must not produce NaN/undefined
const zeroLabel = formatSyncProgressLabel({ processedBlocks: 0, totalBlocks: 0, percent: 100 })
assert.ok(!zeroLabel.includes('NaN'), `unexpected NaN: ${zeroLabel}`)
assert.ok(!zeroLabel.includes('undefined'), `unexpected undefined: ${zeroLabel}`)

// DTO shape: fixture default has no progress; override type-checks and round-trips
assert.equal(makeSyncStatus().syncProgress, null)
const withProgress = makeSyncStatus({ syncProgress: { processedBlocks: 1, totalBlocks: 2, percent: 50 } })
assert.deepEqual(withProgress.syncProgress, { processedBlocks: 1, totalBlocks: 2, percent: 50 })

console.log('format-sync-progress-label: all assertions passed.')
