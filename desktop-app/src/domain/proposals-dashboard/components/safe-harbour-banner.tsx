import { useSafeHarbourActivated } from '@/hooks/use-safe-harbour-status'

/**
 * The bridge state the council's one action produces, shown where the council starts.
 *
 * Rendered only for a Security Council session — no other authority has a lever that answers
 * it, and a bridge-wide alarm nobody on screen can act on is noise. The read lives inside this
 * component so that mounting it is what performs it.
 */
export function SafeHarbourBanner() {
	const activated = useSafeHarbourActivated()
	if (!activated) return null

	return (
		<div className="rounded-xl border border-accent-border bg-highlight-surface px-4 py-3">
			<p className="m-0 text-body font-medium text-emphasis">Safe harbour is already active</p>
			<p className="m-0 mt-1 text-body text-emphasis-soft">
				The bridge is in safe harbour. Another Defcon 1 does not change that.
			</p>
		</div>
	)
}
