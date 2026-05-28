import type { AddressDto, UtxoDto } from '@/api/admin-wallet'
import { groupUtxosByDerivation } from './group-utxos-by-derivation'

export type AddressWithBalanceView = {
	index: number
	address: string
	balanceSats: number
	isUsed: boolean
}

export function composeAddressesWithBalance(addresses: AddressDto[], utxos: UtxoDto[]): AddressWithBalanceView[] {
	const balanceByIndex = groupUtxosByDerivation(utxos)
	return addresses
		.map((addr) => ({
			index: addr.index,
			address: addr.address,
			balanceSats: balanceByIndex.get(addr.index) ?? 0,
			isUsed: addr.isUsed,
		}))
		.filter((row) => row.balanceSats > 0)
}
