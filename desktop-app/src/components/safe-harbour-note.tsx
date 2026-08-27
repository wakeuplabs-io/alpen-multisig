import type { ReactNode } from 'react'
import { useSafeHarbourActivated } from '@/hooks/use-safe-harbour-status'

/**
 * Says that the bridge is already in safe harbour, and nothing else.
 *
 * Renders nothing when the state is inactive or unreadable, so mounting it is what performs the
 * read and no caller has to gate on the answer. The two callers differ only in how much they say
 * about what a second Defcon 1 would cost, which is `children`.
 *
 * Amber, not the `Irreversible` callout's red: this is a fact about the chain, and a second red
 * block above that callout would leave the signer with two alarms of equal weight.
 */
export function SafeHarbourNote({ children }: { children: ReactNode }) {
	const activated = useSafeHarbourActivated()
	if (!activated) return null

	return (
		<div role="status" className="rounded-xl border border-accent-border bg-highlight-surface px-4 py-3">
			<p className="m-0 text-body font-medium text-emphasis">Safe harbour is already active</p>
			<p className="m-0 mt-1 text-body text-emphasis-soft">{children}</p>
		</div>
	)
}
