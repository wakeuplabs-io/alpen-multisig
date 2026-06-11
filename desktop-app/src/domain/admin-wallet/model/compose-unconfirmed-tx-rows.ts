import type { UnconfirmedTxDto } from '@/api/admin-wallet'
import { formatSatPerVb } from '@/domain/fee-selection/model/fee-rate'
import { formatSignedSats } from './format-signed-sats'
import { truncTxid } from './trunc-txid'

export type BumpDisabledReason = 'not-rbf' | 'governance-commit' | null

export type UnconfirmedTxView = {
	txid: string
	shortTxid: string
	/** Signed net amount, e.g. "−40,141 sats". */
	netLabel: string
	/** Absolute fee, e.g. "141 sats" — "Unknown" when a prevout is foreign. */
	feeLabel: string
	/** Current rate, e.g. "1.0 sat/vB" — "—" when the fee is unknown. */
	feeRateLabel: string
	/** ISO timestamp for relativeTime(); null when the indexer gave no last-seen. */
	lastSeenIso: string | null
	isGovernanceCommit: boolean
	canBump: boolean
	bumpDisabledReason: BumpDisabledReason
	currentFeeRateSatPerKvb: number | null
	vsizeVbytes: number
}

/** Maps unconfirmed-tx DTOs to display rows. Order is preserved (backend sorts newest-first). */
export function composeUnconfirmedTxRows(txs: UnconfirmedTxDto[]): UnconfirmedTxView[] {
	return txs.map((tx) => {
		const bumpDisabledReason: BumpDisabledReason = tx.isGovernanceCommit
			? 'governance-commit'
			: !tx.isRbfSignaling
				? 'not-rbf'
				: null
		return {
			txid: tx.txid,
			shortTxid: truncTxid(tx.txid),
			netLabel: formatSignedSats(tx.netSats),
			feeLabel: tx.feeSats !== null ? `${tx.feeSats.toLocaleString()} sats` : 'Unknown',
			feeRateLabel: tx.feeRateSatPerKvb !== null ? `${formatSatPerVb(tx.feeRateSatPerKvb)} sat/vB` : '—',
			lastSeenIso: tx.lastSeenSecs !== null ? new Date(tx.lastSeenSecs * 1000).toISOString() : null,
			isGovernanceCommit: tx.isGovernanceCommit,
			canBump: bumpDisabledReason === null,
			bumpDisabledReason,
			currentFeeRateSatPerKvb: tx.feeRateSatPerKvb,
			vsizeVbytes: tx.vsizeVbytes,
		}
	})
}
