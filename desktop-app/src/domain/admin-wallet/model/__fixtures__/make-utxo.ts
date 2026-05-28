import type { UtxoDto } from '@/api/admin-wallet'

export function makeUtxo(overrides?: Partial<UtxoDto>): UtxoDto {
	return {
		outpoint: { txid: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', vout: 0 },
		valueSats: 100_000,
		scriptPubkeyHex: '0014abcd',
		keychain: 'External',
		derivationIndex: 0,
		confirmations: 1,
		...overrides,
	}
}
