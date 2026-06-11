import { CopyButton } from '@/components/copy-button'
import type { UnconfirmedTxView } from '@/domain/admin-wallet/model/view-models'
import { relativeTime } from '@/domain/admin-wallet/model/relative-time'

export type UnconfirmedTxRowProps = {
	row: UnconfirmedTxView
	isWatchOnly: boolean
	isBumpOpen: boolean
	onToggleBump(): void
}

function bumpTitle(row: UnconfirmedTxView, isWatchOnly: boolean): string | undefined {
	if (isWatchOnly) return 'Hardware wallet required to sign'
	if (row.bumpDisabledReason === 'not-rbf') return 'This transaction does not signal RBF'
	if (row.bumpMethod === 'cpfp') return 'Accelerates the commit+reveal package via a child transaction (CPFP)'
	return undefined
}

function StatusBadge({ row }: { row: UnconfirmedTxView }) {
	if (row.isGovernanceCommit) {
		return (
			<span className="flex-none rounded-full bg-[#eef2ff] px-1.5 py-0.5 font-sans text-[10px] font-medium uppercase tracking-[0.04em] text-[#4f46e5]">
				Governance
			</span>
		)
	}
	if (row.bumpDisabledReason === 'not-rbf') {
		return (
			<span className="flex-none rounded-full bg-[#f3f4f6] px-1.5 py-0.5 font-sans text-[10px] font-medium uppercase tracking-[0.04em] text-[#6b7280]">
				Not replaceable
			</span>
		)
	}
	return (
		<span className="flex-none rounded-full bg-[#ecfdf5] px-1.5 py-0.5 font-sans text-[10px] font-medium uppercase tracking-[0.04em] text-[#047857]">
			RBF
		</span>
	)
}

export function UnconfirmedTxRow({ row, isWatchOnly, isBumpOpen, onToggleBump }: UnconfirmedTxRowProps) {
	const isBumpDisabled = isWatchOnly || !row.canBump
	const lastSeenLabel = row.lastSeenIso !== null ? relativeTime(row.lastSeenIso, new Date()) : null
	const feeNoun = row.usesPackageStats ? 'Package fee' : 'Fee'

	return (
		<div className="py-2.5" data-testid={`e2e-wallet-tx-row-${row.txid}`}>
			<div className="flex items-center justify-between gap-2">
				<span className="flex min-w-0 items-center gap-1.5 font-mono text-[12px] text-[#111827]" title={row.txid}>
					<span className="min-w-0 truncate">{row.shortTxid}</span>
					<CopyButton text={row.txid} variant="icon" />
				</span>
				<span className="flex-none font-mono text-[12px] tabular-nums text-[#111827]">{row.netLabel}</span>
			</div>
			<div className="mt-1 flex items-center justify-between gap-2">
				<span className="flex min-w-0 items-center gap-1.5 text-[11px] text-[#6b7280]">
					<StatusBadge row={row} />
					<span className="truncate">
						{feeNoun} {row.feeLabel} · {row.feeRateLabel}
						{lastSeenLabel !== null ? ` · ${lastSeenLabel}` : ''}
					</span>
				</span>
				<button
					type="button"
					onClick={onToggleBump}
					disabled={isBumpDisabled}
					title={bumpTitle(row, isWatchOnly)}
					aria-expanded={isBumpOpen}
					data-testid={`e2e-wallet-tx-bump-${row.txid}`}
					className={`flex-none rounded-lg border px-2.5 py-1 text-[11px] font-medium transition ${
						isBumpDisabled
							? 'cursor-not-allowed border-[#f3f4f6] text-[#d1d5db]'
							: isBumpOpen
								? 'border-[#111827] bg-[#111827] text-white'
								: 'border-[#e5e7eb] text-[#374151] hover:border-[#111827] hover:text-[#111827]'
					}`}
				>
					{isBumpOpen ? 'Close' : 'Bump fee'}
				</button>
			</div>
		</div>
	)
}
