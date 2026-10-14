//TODO: revise this.
const AVG_BLOCK_SECONDS = 600

/** Shown once the tip is at or past the activation block: the ASM applies the update there (#602). */
export const ACTIVATION_REACHED_LABEL = 'activation block reached, confirming enactment'

function formatDuration(seconds: number): string {
	const days = Math.floor(seconds / 86400)
	const hours = Math.floor((seconds % 86400) / 3600)
	if (days > 0) return `~${days}d ${hours}h`
	const minutes = Math.floor((seconds % 3600) / 60)
	return `~${hours}h ${minutes}m`
}

/**
 * The activation countdown's trailing time label, or `null` while the tip is unknown.
 *
 * There is no "imminent" case: the ASM applies a queued update while processing the activation
 * block itself, so a tip at that block has already passed it, not about to.
 */
export function activationTimeLabel(activationHeight: number, currentHeight: number | null | undefined): string | null {
	if (currentHeight == null) return null
	const blocksRemaining = activationHeight - currentHeight
	if (blocksRemaining <= 0) return ACTIVATION_REACHED_LABEL
	return formatDuration(blocksRemaining * AVG_BLOCK_SECONDS)
}
