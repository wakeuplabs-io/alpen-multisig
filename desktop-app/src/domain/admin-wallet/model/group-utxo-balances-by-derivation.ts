import type { KeychainDto, UtxoDto } from '@/api/admin-wallet'

export type UtxoBalanceBuckets = {
	confirmedSats: number
	unconfirmedSats: number
}

/**
 * Sums UTXO values of one keychain per derivation index, split by confirmation state.
 * Derivation indices repeat across keychains (external 0 and internal 0 are different
 * addresses), so callers must group each keychain separately.
 */
export function groupUtxoBalancesByDerivation(
	utxos: UtxoDto[],
	keychain: KeychainDto = 'External',
): Map<number, UtxoBalanceBuckets> {
	const result = new Map<number, UtxoBalanceBuckets>()
	for (const utxo of utxos) {
		if (utxo.keychain !== keychain) continue
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
