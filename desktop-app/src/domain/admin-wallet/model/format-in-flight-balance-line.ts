/**
 * Returns the "in flight" balance copy — coins spent by transactions this session broadcast (or is
 * still signing) that no sync has seen yet, and that no send can select — or null when there are none.
 */
export function formatInFlightBalanceLine(reservedSats: number): string | null {
	if (!Number.isFinite(reservedSats) || reservedSats <= 0) return null
	return `${reservedSats.toLocaleString()} sats in flight`
}
