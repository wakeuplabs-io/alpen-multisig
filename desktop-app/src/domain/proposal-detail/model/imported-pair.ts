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
 * Whether the orchestrator kept the broadcast state an import reported (#605).
 *
 * A report about a pair the row does not track changes nothing and still answers 200, so the
 * import has to compare the reveal it sent with the one the row tracks to know it was ignored. An
 * import that named no reveal is ignored too once the row tracks a pair: a report that names no
 * reveal does not move a tagged row.
 */
export function importedPairIgnored(importedRevealTxid: string | null, trackedRevealTxid: string | undefined): boolean {
	if (importedRevealTxid === null) return trackedRevealTxid !== undefined
	return importedRevealTxid.toLowerCase() !== trackedRevealTxid?.toLowerCase()
}

export const IMPORTED_PAIR_IGNORED_NOTICE =
	'The imported broadcast state was not recorded: this proposal already tracks a different commit + reveal.'
