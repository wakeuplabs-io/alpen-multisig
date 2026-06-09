import type { AddressDto, UtxoDto } from '@/api/admin-wallet'
import { groupUtxoBalancesByDerivation } from './group-utxo-balances-by-derivation'

export type AddressWithBalanceView = {
	index: number
	address: string
	confirmedSats: number
	unconfirmedSats: number
	/** Total sats at this index (confirmed + unconfirmed). Not used as the primary display amount. */
	balanceSats: number
	isUsed: boolean
}

export function composeAddressesWithBalance(addresses: AddressDto[], utxos: UtxoDto[]): AddressWithBalanceView[] {
	const balanceByIndex = groupUtxoBalancesByDerivation(utxos)
	return addresses
		.map((addr) => {
			const buckets = balanceByIndex.get(addr.index) ?? { confirmedSats: 0, unconfirmedSats: 0 }
			const balanceSats = buckets.confirmedSats + buckets.unconfirmedSats
			return {
				index: addr.index,
				address: addr.address,
				confirmedSats: buckets.confirmedSats,
				unconfirmedSats: buckets.unconfirmedSats,
				balanceSats,
				isUsed: addr.isUsed,
			}
		})
		.filter((row) => row.balanceSats > 0)
}
