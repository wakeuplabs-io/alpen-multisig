import { useState } from 'react'
import { CopyClipboardIcon } from '@/assets/icons'
import type { BroadcastError, BroadcastPhase } from '../model/broadcast-proposal'

type Props = {
	phase: BroadcastPhase
	proposalStatus?: string
	commitTxid?: string
	revealTxid?: string
	error?: BroadcastError | null
}

type Step = { label: string; detail: string }

const STEPS: Step[] = [
	{ label: 'Commit', detail: 'Funds the reveal output (signed locally)' },
	{ label: 'Reveal', detail: 'Carries the action — sent with the commit' },
	{ label: 'Sent', detail: 'Confirmed on Bitcoin — awaiting ASM enactment' },
]

/** Commit (0) + Reveal (1) are broadcast together as one package. */
const BROADCAST_GROUP_LAST_INDEX = 1

function CopyButton({ text }: { text: string }) {
	const [copied, setCopied] = useState(false)

	function handleCopy() {
		void navigator.clipboard.writeText(text).then(() => {
			setCopied(true)
			setTimeout(() => setCopied(false), 2000)
		})
	}

	return (
		<button
			type="button"
			onClick={handleCopy}
			className="inline-flex shrink-0 items-center gap-1 rounded-md border border-[#e5e7eb] bg-white px-2.5 py-1.5 text-label font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
		>
			<CopyClipboardIcon width={12} height={12} />
			{copied ? 'Copied!' : 'Copy'}
		</button>
	)
}

function TxidRow({ label, txid }: { label: string; txid: string }) {
	return (
		<div>
			<p className="mb-1.5 text-mono-sm font-semibold uppercase tracking-wider text-[#9ca3af]">{label}</p>
			<div className="flex items-center gap-2 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
				<span className="min-w-0 flex-1 truncate font-mono text-label text-[#111827]">{txid}</span>
				<CopyButton text={txid} />
			</div>
		</div>
	)
}

export function BroadcastPhaseProgress({ phase, proposalStatus, commitTxid, revealTxid, error }: Props) {
	const isError = phase === 'error'
	const isDone = phase === 'done'
	const isEnacted = proposalStatus === 'enacted'
	const isAwaitingDevice = phase === 'awaiting-device'
	const isAwaitingConfirmation = phase === 'awaiting-confirmation'
	const showTxids = (isDone || isAwaitingConfirmation) && (commitTxid != null || revealTxid != null)

	function stepState(index: number): 'done' | 'active' | 'pending' {
		if (isDone) return 'done'
		// Submitted: Commit + Reveal are broadcast (✓); step 3 is the active "Awaiting block".
		if (isAwaitingConfirmation) return index <= BROADCAST_GROUP_LAST_INDEX ? 'done' : 'active'
		if ((phase === 'broadcasting' || isAwaitingDevice) && index <= BROADCAST_GROUP_LAST_INDEX) return 'active'
		return 'pending'
	}

	function stepLabel(index: number, fallback: string): string {
		if (index === STEPS.length - 1 && isAwaitingConfirmation) return 'Awaiting block'
		return fallback
	}

	function stepDetail(index: number, fallback: string): string {
		if (index === STEPS.length - 1 && isAwaitingConfirmation) {
			return 'Reveal is in the mempool — confirming on Bitcoin. Safe to leave; it keeps confirming.'
		}
		return fallback
	}

	return (
		<div className="overflow-hidden rounded-xl border border-[#e5e7eb] bg-white shadow-sm">
			<div className="border-b border-[#f3f4f6] px-6 py-4">
				<h3 className="m-0 text-body-lg font-semibold text-[#111827]">
					{isDone
						? isEnacted
							? 'Proposal enacted'
							: 'Reveal confirmed — awaiting enactment'
						: isError
							? 'Send failed'
							: isAwaitingDevice
								? 'Waiting for device…'
								: isAwaitingConfirmation
									? 'Submitted — awaiting confirmation…'
									: 'Sending…'}
				</h3>
			</div>

			<div className="p-6">
				{!isError && (
					<div className="mb-6 space-y-3">
						{STEPS.map((step, i) => {
							const state = stepState(i)
							const done = state === 'done'
							const active = state === 'active'

							return (
								<div key={step.label} className="flex items-start gap-3">
									<div className="relative mt-0.5 flex shrink-0 flex-col items-center">
										<div
											className={[
												'flex h-5 w-5 items-center justify-center rounded-full border text-[10px] font-semibold',
												done
													? 'border-[#0f9d7a] bg-[#0f9d7a] text-white'
													: active
														? 'border-[#0f9d7a] bg-white text-[#0f9d7a]'
														: 'border-[#e5e7eb] bg-[#f9fafb] text-[#9ca3af]',
											].join(' ')}
										>
											{done ? '✓' : i + 1}
										</div>
										{i < STEPS.length - 1 && (
											<div className={['mt-1 h-6 w-px', done ? 'bg-[#0f9d7a]' : 'bg-[#e5e7eb]'].join(' ')} />
										)}
									</div>
									<div className="min-w-0 pb-5">
										<p
											className={[
												'm-0 text-body-sm font-medium',
												done ? 'text-[#0f9d7a]' : active ? 'text-[#111827]' : 'text-[#9ca3af]',
											].join(' ')}
										>
											{stepLabel(i, step.label)}
											{active && <span className="ml-2 inline-block h-2 w-2 animate-pulse rounded-full bg-[#0f9d7a]" />}
										</p>
										<p className="m-0 mt-0.5 text-label text-[#9ca3af]">{stepDetail(i, step.detail)}</p>
									</div>
								</div>
							)
						})}
					</div>
				)}

				{showTxids && (
					<div className="space-y-3">
						{commitTxid && <TxidRow label="Commit TXID" txid={commitTxid} />}
						{revealTxid && <TxidRow label="Reveal TXID" txid={revealTxid} />}
					</div>
				)}

				{isError && error && (
					<div className="space-y-3">
						<div className="rounded-lg border border-[#fecaca] bg-[#fef2f2] px-4 py-3">
							<p className="m-0 text-body-sm text-[#b91c1c]">{error.message}</p>
						</div>
						{error.recovery === 'manual-broadcast' && error.commitTxHex != null && error.revealTxHex != null && (
							<div className="rounded-lg border border-[#e5e7eb] bg-[#f9fafb] p-4 space-y-3">
								<p className="m-0 text-body-sm font-medium text-[#111827]">Send manually</p>
								<p className="m-0 text-label text-[#6b7280]">
									Send the commit first, then the reveal, via any Bitcoin node (
									<code className="font-mono text-mono-sm">sendrawtransaction</code>).
								</p>
								<TxidRow label="Commit TX (send first)" txid={error.commitTxHex} />
								<TxidRow label="Reveal TX (send second)" txid={error.revealTxHex} />
							</div>
						)}
					</div>
				)}
			</div>
		</div>
	)
}
