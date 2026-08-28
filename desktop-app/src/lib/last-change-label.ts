/**
 * When a proposal last changed, as a local date and time.
 *
 * Absolute rather than relative on purpose: a stuck bundle is investigated against a node's logs
 * and a block explorer, where a wall-clock time is the thing that can be looked up. It also does
 * not need a ticking clock to stay accurate on a screen nobody is refreshing — which is exactly
 * the screen this line exists for.
 */
export function lastChangeLabel(updatedAtMs: number): string {
	return `Last change: ${new Date(updatedAtMs).toLocaleString()}`
}
