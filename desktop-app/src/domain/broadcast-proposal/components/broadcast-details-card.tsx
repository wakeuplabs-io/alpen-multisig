import { useState, useEffect } from 'react'
import type { PrepareBroadcastResult, Proposal } from '@/api/proposals'
import type { AdminWalletError } from '@/api/admin-wallet'
import { CopyButton } from '@/components/copy-button'
import { SectionLabel } from '@/components/section-label'
import { satsToBtc } from '../model/broadcast-proposal'
import type { BroadcastPhase } from '../model/broadcast-proposal'
import { BroadcastDevicePrompt } from './broadcast-device-prompt'

type AdminWalletInfoView = {
	address: string
	balanceSats: number
}

type Props = {
	bundle: PrepareBroadcastResult
	proposal: Proposal | null
	onBroadcast: () => void
	isBroadcasting: boolean
	canSign?: boolean
	canSignReason?: string
	adminWalletInfo?: AdminWalletInfoView | null
	utxoCount?: number
	lastSyncedAt?: string | null
	syncError?: AdminWalletError | null
	phase?: BroadcastPhase
}

function relativeTime(isoStr: string): string {
	// Handle both ISO-8601 strings and legacy numeric-only strings (bare Unix epoch seconds)
	const ts = isNaN(Number(isoStr)) ? Date.parse(isoStr) : Number(isoStr) * 1000
	const diffSeconds = Math.floor((Date.now() - ts) / 1000)
	if (diffSeconds < 60) return `${diffSeconds}s ago`
	const diffMinutes = Math.floor(diffSeconds / 60)
	if (diffMinutes < 60) return `${diffMinutes}m ago`
	const diffHours = Math.floor(diffMinutes / 60)
	return `${diffHours}h ago`
}

function LastSyncLabel({ lastSyncedAt }: { lastSyncedAt: string }) {
	const [, setTick] = useState(0)

	useEffect(() => {
		const interval = setInterval(() => setTick((t) => t + 1), 15000)
		return () => clearInterval(interval)
	}, [])

	return <span className="text-[12px] text-[#9ca3af]">Last sync: {relativeTime(lastSyncedAt)}</span>
}

export function BroadcastDetailsCard({
	bundle,
	proposal,
	onBroadcast,
	isBroadcasting,
	canSign = true,
	canSignReason,
	adminWalletInfo,
	utxoCount,
	lastSyncedAt,
	syncError,
	phase,
}: Props) {
	const collectedSignatures = proposal?.signatures.length ?? 0
	const requiredSignatures = proposal?.requiredSignatures ?? 0
	const signaturesProgress =
		requiredSignatures === 0 ? 100 : Math.min((collectedSignatures / requiredSignatures) * 100, 100)

	return (
		<div className="overflow-hidden rounded-xl border border-[#e5e7eb] bg-white shadow-sm">
			{phase === 'awaiting-device' && (
				<div className="border-b border-[#f3f4f6] p-6">
					<BroadcastDevicePrompt />
				</div>
			)}
			{proposal && (
				<div className="border-b border-[#f3f4f6] p-6 pb-5">
					<div className="flex items-start justify-between gap-3">
						<div className="min-w-0 flex-1">
							<h2 className="m-0 font-['BIZ_UDPMincho'] text-[26px] leading-[1.2] text-[#0a0a0a]">
								Proposal #{proposal.seqNo}
							</h2>
							<p className="m-0 mt-1 text-[13px] text-[#6b7280]">{proposal.authority}</p>
						</div>
						<span className="inline-flex shrink-0 items-center gap-1.5 rounded-md border border-[#a7f3d0] bg-[#ecfdf5] px-2.5 py-0.75 text-[11px] font-medium whitespace-nowrap text-[#059669]">
							<span className="h-1.5 w-1.5 flex-none rounded-full bg-[#059669]" aria-hidden="true" />
							Quorum reached
						</span>
					</div>

					<div className="mt-4">
						<div className="mb-1.5 flex items-center justify-between gap-3">
							<p className="m-0 text-[13px] font-medium text-[#121212]">Signatures</p>
							<p className="m-0 text-[13px] font-medium text-[#121212]">
								{collectedSignatures} / {requiredSignatures} <span className="font-normal text-[#6b7280]">signed</span>
							</p>
						</div>
						<div className="h-1.5 rounded-full bg-[#ebedf0]">
							<div
								className="h-1.5 rounded-full transition-all"
								style={{ width: `${signaturesProgress}%`, background: '#0f9d7a' }}
							/>
						</div>
					</div>
				</div>
			)}

			<div className="space-y-5 p-6">
				<div>
					<SectionLabel>Commit TX (preview)</SectionLabel>
					<div className="flex items-start gap-2 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
						<span className="min-w-0 flex-1 break-all font-mono text-[12px] leading-relaxed text-[#111827]">
							{bundle.commitAddress}
						</span>
						<CopyButton text={bundle.commitAddress} />
					</div>
					<p className="mt-2 text-[13px] text-[#6b7280]">
						{satsToBtc(bundle.commitAmountSats)} BTC{' '}
						<span className="text-[12px] text-[#9ca3af]">({bundle.commitAmountSats.toLocaleString()} sats)</span>
					</p>
				</div>

				<div>
					<SectionLabel>Reveal TX</SectionLabel>
					<p className="text-[13px] text-[#6b7280]">
						Signed locally and broadcast in the same package as the commit — no separate confirmation wait.
					</p>
				</div>

				<div className="flex items-center justify-between rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
					<span className="text-[11px] font-semibold uppercase tracking-wider text-[#9ca3af]">Estimated fee</span>
					<span className="text-[13px] font-medium text-[#111827]">
						{bundle.estimatedFeeSats.toLocaleString()} sats
					</span>
				</div>

				{adminWalletInfo != null && (
					<div>
						<SectionLabel>Funding Source</SectionLabel>
						<div className="flex items-start gap-2 rounded-lg border border-[#e5e7eb] bg-[#f9fafb] px-3 py-2.5">
							<span
								data-testid="e2e-admin-wallet-external-address-0"
								className="min-w-0 flex-1 break-all font-mono text-[12px] leading-relaxed text-[#111827]"
							>
								{adminWalletInfo.address}
							</span>
							<CopyButton text={adminWalletInfo.address} />
						</div>
						<p className="mt-2 text-[13px] text-[#6b7280]">
							Admin Wallet (BDK){' '}
							<span className="text-[12px] text-[#9ca3af]">({adminWalletInfo.balanceSats.toLocaleString()} sats)</span>
						</p>
						{utxoCount !== undefined && <p className="mt-1 text-[12px] text-[#9ca3af]">UTXOs: {utxoCount}</p>}
						{syncError != null ? (
							<p className="mt-1 text-[12px] text-[#ef4444]">
								Sync error: {'message' in syncError ? syncError.message : syncError.type}
							</p>
						) : lastSyncedAt != null ? (
							<p className="mt-1">
								<LastSyncLabel lastSyncedAt={lastSyncedAt} />
							</p>
						) : null}
					</div>
				)}

				<button
					type="button"
					data-testid="e2e-broadcast-confirm"
					disabled={isBroadcasting || !canSign}
					onClick={onBroadcast}
					className="w-full rounded-xl border border-[#111827] bg-[#111827] px-4 py-2.5 text-sm font-medium text-white transition hover:bg-black disabled:cursor-not-allowed disabled:opacity-60"
				>
					{isBroadcasting ? 'Broadcasting…' : 'Confirm & Broadcast'}
				</button>
				{!canSign && (
					<p className="mt-2 text-center text-[12px] text-[#6b7280]">
						{canSignReason ?? 'Hardware wallet required to sign'}
					</p>
				)}
			</div>
		</div>
	)
}
