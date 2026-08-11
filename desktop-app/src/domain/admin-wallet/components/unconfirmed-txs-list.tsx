import { useState } from 'react'
import type { AdminWalletError } from '@/domain/admin-wallet/model/types'
import type { UnconfirmedTxView } from '@/domain/admin-wallet/model/view-models'
import {
	FALLBACK_MAX_SAT_PER_KVB,
	FALLBACK_MIN_RELAY_SAT_PER_KVB,
	effectiveMaxBumpRate,
	minBumpRateSatPerKvb,
	suggestedBumpRateSatPerKvb,
} from '@/domain/admin-wallet/model/bump-fee-rate'
import { formatAdminWalletError } from '@/domain/admin-wallet/model/format-admin-wallet-error'
import { useBumpFee } from '@/domain/admin-wallet/hooks/use-bump-fee'
import { useFeePresets } from '@/domain/fee-selection/hooks/use-fee-presets'
import { UnconfirmedTxRow } from './unconfirmed-tx-row'
import { BumpFeeForm } from './bump-fee-form'

export type UnconfirmedTxsListProps = {
	rows: UnconfirmedTxView[] | null
	isLoading: boolean
	error: AdminWalletError | null
	isExpanded: boolean
	onToggle(): void
	isWatchOnly: boolean
	/** Called after a successful bump so the panel re-syncs and re-reads. */
	onAfterBump(): void
}

export function UnconfirmedTxsList({
	rows,
	isLoading,
	error,
	isExpanded,
	onToggle,
	isWatchOnly,
	onAfterBump,
}: UnconfirmedTxsListProps) {
	const count = rows?.length ?? 0
	const [openBumpTxid, setOpenBumpTxid] = useState<string | null>(null)
	const bumpHook = useBumpFee()
	const feePresets = useFeePresets()

	const fastPresetSatPerKvb = feePresets.status === 'ready' ? feePresets.presets.fast.satPerKvb : null
	const minRelaySatPerKvb =
		feePresets.status === 'ready' ? feePresets.presets.minRelaySatPerKvb : FALLBACK_MIN_RELAY_SAT_PER_KVB
	const maxSatPerKvb = feePresets.status === 'ready' ? feePresets.presets.maxSatPerKvb : FALLBACK_MAX_SAT_PER_KVB

	function handleToggleBump(txid: string) {
		bumpHook.reset()
		setOpenBumpTxid((open) => (open === txid ? null : txid))
	}

	function handleConfirm(row: UnconfirmedTxView, satPerKvb: number) {
		void bumpHook.bump(row.txid, satPerKvb).then((ok) => {
			if (ok) onAfterBump()
		})
	}

	function handleCloseForm() {
		bumpHook.reset()
		setOpenBumpTxid(null)
	}

	return (
		<div>
			<button
				type="button"
				onClick={onToggle}
				className="flex w-full items-center justify-between px-0 py-2.5 text-left hover:bg-transparent"
				data-testid="e2e-wallet-transactions-toggle"
			>
				<span className="flex items-center font-sans text-body-sm font-medium text-[#374151]">
					Pending transactions
					<span className="ml-2 rounded-full bg-[#f3f4f6] px-2 py-0.5 text-mono-sm tabular-nums text-[#6b7280]">
						{count}
					</span>
				</span>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					strokeWidth="2"
					strokeLinecap="round"
					strokeLinejoin="round"
					aria-hidden="true"
					className={`flex-none text-[#9ca3af] transition-transform duration-150 ${isExpanded ? 'rotate-180' : 'rotate-0'}`}
				>
					<polyline points="6 9 12 15 18 9" />
				</svg>
			</button>

			{isExpanded && (
				<div>
					{isLoading && rows === null && (
						<div className="space-y-1.5 py-3">
							{[0, 1].map((i) => (
								<div key={i} className="h-4 w-full animate-pulse rounded bg-[#e5e7eb]" />
							))}
						</div>
					)}

					{error !== null && rows === null && !isLoading && (
						<div className="py-3 text-label text-danger">{formatAdminWalletError(error).body}</div>
					)}

					{rows !== null && rows.length === 0 && !isLoading && error === null && (
						<div className="py-3 text-label text-[#9ca3af]" data-testid="e2e-wallet-transactions-empty">
							No pending transactions
						</div>
					)}

					{rows !== null && rows.length > 0 && (
						<div className="divide-y divide-[#f3f4f6]" data-testid="e2e-wallet-transactions-list">
							{rows.map((row) => {
								const minBump = minBumpRateSatPerKvb(row.currentFeeRateSatPerKvb, minRelaySatPerKvb)
								const rowMax = effectiveMaxBumpRate(maxSatPerKvb, row.maxBumpRateSatPerKvb)
								return (
									<div key={row.txid}>
										<UnconfirmedTxRow
											row={row}
											isWatchOnly={isWatchOnly}
											isBumpOpen={openBumpTxid === row.txid}
											onToggleBump={() => handleToggleBump(row.txid)}
										/>
										{openBumpTxid === row.txid && row.bumpMethod !== null && (
											<BumpFeeForm
												method={row.bumpMethod}
												currentFeeRateLabel={row.feeRateLabel}
												currentFeeSats={row.currentFeeSats}
												minBumpSatPerKvb={minBump}
												suggestedSatPerKvb={Math.min(rowMax, suggestedBumpRateSatPerKvb(fastPresetSatPerKvb, minBump))}
												maxSatPerKvb={rowMax}
												vsizeVbytes={row.vsizeVbytes}
												targetTxid={row.txid}
												state={bumpHook.state}
												onConfirm={(satPerKvb) => handleConfirm(row, satPerKvb)}
												onClose={handleCloseForm}
											/>
										)}
									</div>
								)
							})}
						</div>
					)}
				</div>
			)}
		</div>
	)
}
