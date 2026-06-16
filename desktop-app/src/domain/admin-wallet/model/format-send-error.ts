// Phase 6 (PRD §4.3.5.1) — exact PRD copy for the Send destination field.
//
// These two strings are PRD MUSTs and are asserted byte-for-byte in
// __tests__/format-send-error.test.ts. The backend stays copy-free: it reports
// a typed reason + the expected network word, and this module is the single
// place the user-facing sentences live.

import type { SendAddressInvalidReason } from '@/api/admin-wallet'

/**
 * PRD §4.3.5.1 destination error copy:
 * - not a parseable address → "Destination must be a bitcoin address."
 * - wrong network → "Destination must be a [correct network] bitcoin address."
 */
export function formatSendDestinationError(reason: SendAddressInvalidReason, expectedNetwork: string): string {
	if (reason === 'wrong-network') {
		return `Destination must be a ${expectedNetwork} bitcoin address.`
	}
	return 'Destination must be a bitcoin address.'
}

/** Neutral copy when the validation IPC itself is unreachable (blocks Confirm). */
export const SEND_DESTINATION_UNAVAILABLE_COPY = 'Could not validate the address — check the wallet session.'

/** PRD §4.3.5.2 exact copy for the over-balance amount error. */
export const SEND_INSUFFICIENT_FUNDS_COPY = 'Insufficient funds'
