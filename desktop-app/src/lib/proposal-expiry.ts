const HOUR_MS = 60 * 60 * 1000
const DAY_MS = 24 * HOUR_MS

export type ExpiryUrgency = 'none' | 'warning' | 'urgent'

/**
 * How loudly the pending countdown should speak, given the time left and the proposal's lifetime.
 *
 * The thresholds are 24 h (warning) and 1 h (urgent), capped at the last quarter of the lifetime so
 * that a short window does not warn from creation (#551): a signal that is always on carries nothing.
 */
export function expiryUrgency(timeLeftMs: number, lifetimeMs: number): ExpiryUrgency {
	const quarter = lifetimeMs / 4
	if (timeLeftMs < Math.min(HOUR_MS, quarter)) return 'urgent'
	if (timeLeftMs < Math.min(DAY_MS, quarter)) return 'warning'
	return 'none'
}

/** The time left before a pending proposal expires, at the two most significant units. */
export function formatExpiryTimeLeft(ms: number): string {
	if (ms <= 0) return 'Expired'
	const totalSeconds = Math.floor(ms / 1000)
	const days = Math.floor(totalSeconds / 86400)
	const hours = Math.floor((totalSeconds % 86400) / 3600)
	const minutes = Math.floor((totalSeconds % 3600) / 60)
	if (days > 0) return `Expires in ${days} d ${hours} h`
	if (hours > 0) return `Expires in ${hours} h ${minutes} m`
	return `Expires in ${minutes} m`
}
