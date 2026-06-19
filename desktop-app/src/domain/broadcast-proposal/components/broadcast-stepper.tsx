import type { BroadcastPhase } from '../model/broadcast-proposal'

type StepDef = { label: string; short: string }

const STEPS: StepDef[] = [
	{ label: 'Fee Selection', short: 'Fee' },
	{ label: 'Commit', short: 'Commit' },
	{ label: 'Reveal', short: 'Reveal' },
	{ label: 'Confirmed', short: 'Done' },
]

function phaseToStepIndex(phase: BroadcastPhase): number {
	switch (phase) {
		case 'idle':
		case 'preparing':
		case 'confirming':
			return 0
		case 'awaiting-device':
		case 'broadcasting':
			return 1
		case 'awaiting-confirmation':
			return 3
		case 'done':
			return 3
		case 'error':
			return -1
	}
}

type Props = {
	phase: BroadcastPhase
}

export function BroadcastStepper({ phase }: Props) {
	const activeIndex = phaseToStepIndex(phase)
	const isError = phase === 'error'
	const allDone = phase === 'done'

	return (
		<div className="flex items-center gap-0">
			{STEPS.map((step, i) => {
				const done = allDone || activeIndex > i
				const active = !allDone && activeIndex === i && !isError

				return (
					<div key={step.label} className="flex items-center">
						{i > 0 && <div className={['h-px w-6 sm:w-10', done ? 'bg-[#0f9d7a]' : 'bg-[#e5e7eb]'].join(' ')} />}
						<div className="flex items-center gap-1.5">
							<div
								className={[
									'flex h-6 w-6 items-center justify-center rounded-full text-[11px] font-semibold',
									done
										? 'bg-[#0f9d7a] text-white'
										: active
											? 'border-2 border-[#0f9d7a] bg-white text-[#0f9d7a]'
											: 'border border-[#e5e7eb] bg-[#f9fafb] text-[#9ca3af]',
								].join(' ')}
							>
								{done ? '✓' : i + 1}
							</div>
							<span
								className={[
									'text-label font-medium whitespace-nowrap',
									done ? 'text-[#0f9d7a]' : active ? 'text-[#111827]' : 'text-[#9ca3af]',
								].join(' ')}
							>
								{step.label}
							</span>
						</div>
					</div>
				)
			})}
		</div>
	)
}
