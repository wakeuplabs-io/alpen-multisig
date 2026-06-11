import type { AddressDto, KeychainDto, UtxoDto } from '@/api/admin-wallet'
import { groupUtxoBalancesByDerivation } from './group-utxo-balances-by-derivation'

export type AddressWithBalanceView = {
	index: number
	address: string
	/** Which keychain the address belongs to — 'Internal' rows are change addresses. */
	keychain: KeychainDto
	confirmedSats: number
	unconfirmedSats: number
	/** Total sats at this index (confirmed + unconfirmed). Not used as the primary display amount. */
	balanceSats: number
	isUsed: boolean
}

export type AddressesByKeychain = {
	external: AddressDto[]
	internal: AddressDto[]
}

/**
 * Joins the address windows of BOTH keychains against the UTXO set, keeping only rows that hold
 * balance. Change (internal) addresses must be included: after a spend, the remaining funds sit
 * in change UTXOs — counted by the header balance — and hiding them made the list show
 * "No addresses with balance" while the wallet held funds.
 */
export function composeAddressesWithBalance(
	addresses: AddressesByKeychain,
	utxos: UtxoDto[],
): AddressWithBalanceView[] {
	const toRows = (addrs: AddressDto[], keychain: KeychainDto): AddressWithBalanceView[] => {
		const balanceByIndex = groupUtxoBalancesByDerivation(utxos, keychain)
		return addrs.map((addr) => {
			const buckets = balanceByIndex.get(addr.index) ?? { confirmedSats: 0, unconfirmedSats: 0 }
			const balanceSats = buckets.confirmedSats + buckets.unconfirmedSats
			return {
				index: addr.index,
				address: addr.address,
				keychain,
				confirmedSats: buckets.confirmedSats,
				unconfirmedSats: buckets.unconfirmedSats,
				balanceSats,
				isUsed: addr.isUsed,
			}
		})
	}

	return [...toRows(addresses.external, 'External'), ...toRows(addresses.internal, 'Internal')].filter(
		(row) => row.balanceSats > 0,
	)
}
