/**
 * The txid an import reports, or `null` for none (#605). A txid the row already has is left out —
 * unless the import also advances the broadcast status: the orchestrator ignores a report that
 * names no reveal once the row tracks one, so a status update for the tracked pair must name it.
 */
export function importedTxid(raw: unknown, existing: string | null, advancesStatus: boolean): string | null {
	if (typeof raw !== 'string') return null
	if (raw === existing && !advancesStatus) return null
	return raw
}

/**
 * What to tell the signer after an import, or `null` when the row took it as reported (#605).
 *
 * A report about a pair the row does not track still answers 200 and leaves the row where it is;
 * the orchestrator lists the pair beside the tracked one, and confirms it if it is the reveal the
 * ASM executed (a pair sent by the manual fallback, say). An import that named no reveal changes
 * nothing once the row tracks a pair: a report that names no reveal does not move a tagged row.
 */
export function importNotice(importedRevealTxid: string | null, trackedRevealTxid: string | undefined): string | null {
	if (importedRevealTxid === null) {
		return trackedRevealTxid === undefined ? null : IMPORT_NOT_RECORDED_NOTICE
	}
	if (importedRevealTxid.toLowerCase() === trackedRevealTxid?.toLowerCase()) return null
	return IMPORTED_PAIR_LISTED_NOTICE
}

export const IMPORT_NOT_RECORDED_NOTICE =
	'The imported broadcast state was not recorded: it names no reveal, and this proposal already tracks a commit + reveal.'

export const IMPORTED_PAIR_LISTED_NOTICE =
	'The imported commit + reveal was kept beside the one this proposal tracks. It is confirmed only if it is the reveal the ASM executed.'
