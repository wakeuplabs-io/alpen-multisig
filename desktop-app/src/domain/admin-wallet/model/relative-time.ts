export function relativeTime(isoString: string, now: Date): string {
	const ts = Date.parse(isoString)
	if (isNaN(ts)) return '—'
	const diffSeconds = Math.floor((now.getTime() - ts) / 1000)
	if (diffSeconds < 0) return 'just now'
	if (diffSeconds < 60) return `${diffSeconds}s ago`
	const diffMinutes = Math.floor(diffSeconds / 60)
	if (diffMinutes < 60) return `${diffMinutes}m ago`
	const diffHours = Math.floor(diffMinutes / 60)
	return `${diffHours}h ago`
}
