/**
 * The time left on the session, as `HH:MM:SS`.
 *
 * A session lasts up to 24 hours from sign-in (#582), so the label always carries an hours field.
 * Partial seconds are dropped rather than rounded up, and `null` (no session) renders as a
 * placeholder of the same width so the chip does not jump when a session starts or ends.
 */
export function formatSessionCountdown(remainingMs: number | null): string {
	if (remainingMs === null) return '--:--:--'
	const totalSeconds = Math.floor(Math.max(0, remainingMs) / 1_000)
	const hours = Math.floor(totalSeconds / 3_600)
	const minutes = Math.floor((totalSeconds % 3_600) / 60)
	const seconds = totalSeconds % 60
	return [hours, minutes, seconds].map((part) => String(part).padStart(2, '0')).join(':')
}
