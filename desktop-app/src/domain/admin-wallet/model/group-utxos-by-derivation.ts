import type { UtxoDto } from '@/api/admin-wallet'

export function groupUtxosByDerivation(utxos: UtxoDto[], opts?: { includeInternal?: boolean }): Map<number, number> {
	const result = new Map<number, number>()
	for (const utxo of utxos) {
		if (utxo.keychain === 'Internal' && !opts?.includeInternal) continue
		const current = result.get(utxo.derivationIndex) ?? 0
		result.set(utxo.derivationIndex, current + utxo.valueSats)
	}
	return result
}
