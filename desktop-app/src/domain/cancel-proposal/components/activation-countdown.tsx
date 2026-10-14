import { activationTimeLabel } from '@/lib/activation-time-label'

type Props = {
	activationHeight: number
	currentHeight?: number | null
}

export function ActivationCountdown({ activationHeight, currentHeight }: Props) {
	const timeLabel = activationTimeLabel(activationHeight, currentHeight)

	return (
		<div className="inline-flex items-center gap-1.5 text-body-sm text-emphasis-soft">
			<span aria-hidden="true">⏱</span>
			<span>
				Activation in block <span className="font-mono font-medium">{activationHeight.toLocaleString()}</span>
				{currentHeight != null && (
					<span className="text-emphasis-soft">
						{' '}
						· current block <span className="font-mono font-medium">{currentHeight.toLocaleString()}</span>
					</span>
				)}
				{timeLabel !== null && <> · {timeLabel}</>}
			</span>
		</div>
	)
}
