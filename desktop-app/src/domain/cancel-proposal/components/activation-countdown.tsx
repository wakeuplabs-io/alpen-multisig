type Props = {
	activationHeight: number
	currentHeight?: number | null
}

//TODO: revise this.
const AVG_BLOCK_SECONDS = 600

function formatDuration(seconds: number): string {
	if (seconds <= 0) return 'imminent'
	const days = Math.floor(seconds / 86400)
	const hours = Math.floor((seconds % 86400) / 3600)
	if (days > 0) return `~${days}d ${hours}h`
	const minutes = Math.floor((seconds % 3600) / 60)
	return `~${hours}h ${minutes}m`
}

export function ActivationCountdown({ activationHeight, currentHeight }: Props) {
	const blocksRemaining = currentHeight != null ? Math.max(0, activationHeight - currentHeight) : null
	const timeLabel = blocksRemaining !== null ? formatDuration(blocksRemaining * AVG_BLOCK_SECONDS) : null

	return (
		<div className="inline-flex items-center gap-1.5 text-body-sm text-[#d97706]">
			<span aria-hidden="true">⏱</span>
			<span>
				Activation in block <span className="font-mono font-medium">{activationHeight.toLocaleString()}</span>
				{currentHeight != null && (
					<span className="text-[#b45309]">
						{' '}
						· current block <span className="font-mono font-medium">{currentHeight.toLocaleString()}</span>
					</span>
				)}
				{timeLabel !== null && <> · {timeLabel}</>}
			</span>
		</div>
	)
}
