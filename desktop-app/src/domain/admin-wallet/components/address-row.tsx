import { truncAddress } from '@/domain/admin-wallet/model/trunc-address'
import { formatBtcFromSats } from '@/domain/admin-wallet/model/format-btc-from-sats'

export type AddressRowProps = {
	index: number
	address: string
	balanceSats: number
	isUsed: boolean
}

export function AddressRow({ index, address, balanceSats, isUsed }: AddressRowProps) {
	return (
		<tr className={isUsed ? 'text-[#111827]' : 'text-[#9ca3af]'}>
			<td className="py-1.5 pl-4 pr-3 text-[12px] font-mono tabular-nums">{index}</td>
			<td className="px-3 py-1.5 text-[12px] font-mono" title={address}>
				{truncAddress(address)}
			</td>
			<td className="px-3 py-1.5 text-right text-[12px] tabular-nums">{formatBtcFromSats(balanceSats)} BTC</td>
		</tr>
	)
}
