import type { BroadcastPhase } from '../model/broadcast-proposal'

type Props = {
	phase: BroadcastPhase
	commitTxid?: string
	revealTxid?: string
	error?: string | null
}

const PHASES: { key: BroadcastPhase; label: string }[] = [
	{ key: 'confirming', label: 'Prepare' },
	{ key: 'broadcasting', label: 'Commit' },
	{ key: 'done', label: 'Reveal' },
]

const PHASE_ORDER: BroadcastPhase[] = ['idle', 'preparing', 'confirming', 'broadcasting', 'done', 'error']

function phaseIndex(phase: BroadcastPhase) {
	return PHASE_ORDER.indexOf(phase)
}

export function BroadcastPhaseProgress({ phase, commitTxid, revealTxid, error }: Props) {
	const currentIndex = phaseIndex(phase)
	const isError = phase === 'error'

	return (
		<div className="rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-sm">
			<h3 className="mb-4 text-base font-semibold text-[#111827]">Broadcast Progress</h3>
			<div className="flex items-center gap-2">
				{PHASES.map((step, i) => {
					const stepIndex = phaseIndex(step.key)
					const isDone = currentIndex >= stepIndex && !isError
					const isActive = currentIndex === stepIndex && !isError

					return (
						<div key={step.key} className="flex flex-1 flex-col items-center gap-1">
							<div
								className={[
									'h-2 w-full rounded-full',
									isDone ? 'bg-[#0f9d7a]' : isActive ? 'bg-[#0f9d7a] opacity-50' : 'bg-[#e5e7eb]',
									isError && i === 0 ? 'bg-red-400' : '',
								].join(' ')}
							/>
							<span className="text-[11px] text-[#6b7280]">{step.label}</span>
						</div>
					)
				})}
			</div>
			{phase === 'done' && (
				<div className="mt-4 space-y-2 text-xs">
					{commitTxid && (
						<p className="text-[#6b7280]">
							Commit txid: <span className="font-mono text-[#111827]">{commitTxid}</span>
						</p>
					)}
					{revealTxid && (
						<p className="text-[#6b7280]">
							Reveal txid: <span className="font-mono text-[#111827]">{revealTxid}</span>
						</p>
					)}
				</div>
			)}
			{isError && error && <p className="mt-3 text-sm text-red-600">{error}</p>}
		</div>
	)
}
