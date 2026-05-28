export function formatBtcFromSats(sats: number): string {
	if (!Number.isFinite(sats)) return '—'
	const abs = Math.abs(sats)
	let decimals: number
	if (abs < 100_000_000) {
		decimals = 8
	} else if (abs < 10_000_000_000) {
		decimals = 6
	} else {
		decimals = 4
	}
	const btc = sats / 100_000_000
	return btc.toFixed(decimals)
}
