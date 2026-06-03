import { formatSignedSats } from '@/domain/admin-wallet/model/format-signed-sats'

/** Returns user-facing unconfirmed balance copy, or null when there is no pending net movement. */
export function formatUnconfirmedBalanceLine(unconfirmedSats: number): string | null {
	if (unconfirmedSats === 0) return null
	return `${formatSignedSats(unconfirmedSats)} unconfirmed`
}
