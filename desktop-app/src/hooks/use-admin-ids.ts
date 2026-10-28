import { useEffect, useState } from 'react'
import { adminIdsForPubkeys } from '@/api/admin-id'
import { normalizePubkey } from '@/lib/pubkey'

/**
 * Admin IDs already resolved in this process, by normalized signer key. A key's Admin ID cannot change while the
 * app runs (the network is process-wide), so entries are never evicted. `''` marks a key that has none: not a
 * valid compressed key, or the derivation failed.
 */
const resolved = new Map<string, string>()
const queued = new Set<string>()
const listeners = new Set<() => void>()
let flushScheduled = false

/** The distinct normalized keys of `pubkeys` that `known` has no entry for, sorted so the result is stable. */
export function missingPubkeys(pubkeys: readonly string[], known: ReadonlyMap<string, string>): string[] {
	return [...new Set(pubkeys.map(normalizePubkey))].filter((key) => !known.has(key)).sort()
}

/** What a batch answer stores: the Admin ID for each requested key, `''` for one the answer leaves out. */
export function entriesForBatch(batch: readonly string[], answer: Record<string, string> | null): [string, string][] {
	return batch.map((key) => [key, answer?.[key] ?? ''])
}

async function flush(): Promise<void> {
	flushScheduled = false
	const batch = [...queued]
	queued.clear()
	const result = await adminIdsForPubkeys(batch)
	for (const [key, adminId] of entriesForBatch(batch, result.ok ? result.data : null)) resolved.set(key, adminId)
	for (const notify of listeners) notify()
}

/** Every row a screen renders asks in the same commit, so one microtask turns them into a single IPC call. */
function request(keys: readonly string[]): void {
	for (const key of keys) queued.add(key)
	if (!flushScheduled) {
		flushScheduled = true
		queueMicrotask(() => void flush())
	}
}

/**
 * Admin IDs for `pubkeys`, keyed by `normalizePubkey`. A key is absent while it is being resolved and `''` when it
 * has no Admin ID.
 */
export function useAdminIds(pubkeys: readonly string[]): ReadonlyMap<string, string> {
	const [, setVersion] = useState(0)
	// A string, so the effect depends on which keys are missing rather than on the caller's array identity.
	const missing = missingPubkeys(pubkeys, resolved).join(',')

	useEffect(() => {
		const notify = () => setVersion((version) => version + 1)
		listeners.add(notify)
		if (missing !== '') request(missing.split(','))
		return () => {
			listeners.delete(notify)
		}
	}, [missing])

	return resolved
}
