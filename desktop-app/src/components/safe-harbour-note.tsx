import type { ReactNode } from 'react'

/**
 * Says that the bridge is already in safe harbour, and nothing else.
 *
 * Presentational: the caller decides whether the state warrants it, and says how much to add
 * about what a second Defcon 1 would cost. Amber, not the `Irreversible` callout's red — this is
 * a fact about the chain, and a second red block above that callout would leave the signer with
 * two alarms of equal weight.
 */
export function SafeHarbourNote({ children }: { children: ReactNode }) {
	return (
		<div role="status" className="rounded-xl border border-accent-border bg-highlight-surface px-4 py-3">
			<p className="m-0 text-body font-medium text-emphasis">Safe harbour is already active</p>
			<p className="m-0 mt-1 text-body text-emphasis-soft">{children}</p>
		</div>
	)
}
