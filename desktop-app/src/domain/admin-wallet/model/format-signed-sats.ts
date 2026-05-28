export function formatSignedSats(sats: number): string {
	if (!Number.isFinite(sats)) return '—'
	if (sats < 0) {
		return `−${Math.abs(sats).toLocaleString()} sats`
	}
	return `+${sats.toLocaleString()} sats`
}
