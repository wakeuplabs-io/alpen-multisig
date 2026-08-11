import type { BumpMethodDto, UnconfirmedTxDto } from '@/api/admin-wallet'
import { formatSatPerVb } from '@/domain/fee-selection/model/fee-rate'
import { formatSignedSats } from './format-signed-sats'
import { truncTxid } from './trunc-txid'

export type BumpDisabledReason = 'not-rbf' | 'cpfp-stats-unavailable' | null

export type UnconfirmedTxView = {
	txid: string
	shortTxid: string
	/** Signed net amount, e.g. "−40,141 sats". */
	netLabel: string
	/** Absolute fee, e.g. "141 sats" — package fee for governance rows; "Unknown" when foreign. */
	feeLabel: string
	/** Current rate, e.g. "1.0 sat/vB" — package rate for governance rows; "—" when unknown. */
	feeRateLabel: string
	/** True when feeLabel/feeRateLabel describe the commit+reveal package (CPFP rows). */
	usesPackageStats: boolean
	/** ISO timestamp for relativeTime(); null when the indexer gave no last-seen. */
	lastSeenIso: string | null
	isGovernanceCommit: boolean
	/** 'rbf' | 'cpfp' from the backend; null when the tx cannot be bumped. */
	bumpMethod: BumpMethodDto | null
	canBump: boolean
	bumpDisabledReason: BumpDisabledReason
	/** Rate the bump must exceed — the package rate for CPFP rows. */
	currentFeeRateSatPerKvb: number | null
	/** Fee already paid — the package fee for CPFP rows (drives the child-fee estimate). */
	currentFeeSats: number | null
	/** vsize the bump rate applies to — the package vsize for CPFP rows. */
	vsizeVbytes: number
	/** Highest rate this row can be bumped to; null when only the general ceiling applies. */
	maxBumpRateSatPerKvb: number | null
}

/** Maps unconfirmed-tx DTOs to display rows. Order is preserved (backend sorts newest-first). */
export function composeUnconfirmedTxRows(txs: UnconfirmedTxDto[]): UnconfirmedTxView[] {
	return txs.map((tx) => {
		const usesPackageStats = tx.bumpMethod === 'cpfp' && tx.packageFeeSats !== null
		const effectiveFeeSats = usesPackageStats ? tx.packageFeeSats : tx.feeSats
		const effectiveRate = usesPackageStats ? tx.packageFeeRateSatPerKvb : tx.feeRateSatPerKvb
		const effectiveVsize = usesPackageStats && tx.packageVsizeVbytes !== null ? tx.packageVsizeVbytes : tx.vsizeVbytes

		// F-009: CPFP rows require package stats to bump. If the reveal is not in the
		// wallet graph yet (packageFeeSats is null), disable bump until sync completes.
		const cpfpStatsUnavailable = tx.bumpMethod === 'cpfp' && tx.packageFeeSats === null
		const canBump = tx.bumpMethod !== null && !cpfpStatsUnavailable
		const bumpDisabledReason: BumpDisabledReason = cpfpStatsUnavailable
			? 'cpfp-stats-unavailable'
			: tx.bumpMethod === null
				? 'not-rbf'
				: null

		return {
			txid: tx.txid,
			shortTxid: truncTxid(tx.txid),
			netLabel: formatSignedSats(tx.netSats),
			feeLabel: effectiveFeeSats !== null ? `${effectiveFeeSats.toLocaleString()} sats` : 'Unknown',
			feeRateLabel: effectiveRate !== null ? `${formatSatPerVb(effectiveRate)} sat/vB` : '—',
			usesPackageStats,
			lastSeenIso: tx.lastSeenSecs !== null ? new Date(tx.lastSeenSecs * 1000).toISOString() : null,
			isGovernanceCommit: tx.isGovernanceCommit,
			bumpMethod: tx.bumpMethod,
			canBump,
			bumpDisabledReason,
			currentFeeRateSatPerKvb: effectiveRate,
			currentFeeSats: effectiveFeeSats,
			vsizeVbytes: effectiveVsize,
			maxBumpRateSatPerKvb: tx.maxBumpRateSatPerKvb,
		}
	})
}
