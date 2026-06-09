import type { UtxoDto } from '@/api/admin-wallet'

export type UtxoBalanceBuckets = {
	confirmedSats: number
	unconfirmedSats: number
}

/** Sums external UTXO values per derivation index, split by confirmation state. */
export function groupUtxoBalancesByDerivation(
	utxos: UtxoDto[],
	opts?: { includeInternal?: boolean },
): Map<number, UtxoBalanceBuckets> {
	const result = new Map<number, UtxoBalanceBuckets>()
	for (const utxo of utxos) {
		if (utxo.keychain === 'Internal' && !opts?.includeInternal) continue
		const current = result.get(utxo.derivationIndex) ?? { confirmedSats: 0, unconfirmedSats: 0 }
		if (utxo.confirmations === 0) {
			current.unconfirmedSats += utxo.valueSats
		} else {
			current.confirmedSats += utxo.valueSats
		}
		result.set(utxo.derivationIndex, current)
	}
	return result
}
