import { CopyButton } from '@/components/copy-button'
import { truncAddress } from '@/domain/admin-wallet/model/trunc-address'
import { formatBtcFromSats } from '@/domain/admin-wallet/model/format-btc-from-sats'
import { formatUnconfirmedBalanceLine } from '@/domain/admin-wallet/model/format-unconfirmed-balance-line'

export type AddressRowProps = {
	index: number
	address: string
	confirmedSats: number
	unconfirmedSats: number
	isUsed: boolean
}

export function AddressRow({ index, address, confirmedSats, unconfirmedSats, isUsed }: AddressRowProps) {
	const unconfirmedLine = formatUnconfirmedBalanceLine(unconfirmedSats)

	return (
		<tr className={isUsed ? 'text-[#111827]' : 'text-[#9ca3af]'}>
			<td className="py-1.5 pl-4 pr-3 text-[12px] font-mono tabular-nums">{index}</td>
			<td className="px-3 py-1.5 text-[12px] font-mono" title={address}>
				<div className="flex items-center gap-1.5">
					<span className="min-w-0 truncate">{truncAddress(address)}</span>
					<CopyButton text={address} />
				</div>
			</td>
			<td className="px-3 py-1.5 text-right text-[12px] tabular-nums">
				<div>{formatBtcFromSats(confirmedSats)} BTC</div>
				{unconfirmedLine !== null && (
					<div className="mt-0.5 text-[11px] font-normal text-[#9ca3af]">{unconfirmedLine}</div>
				)}
			</td>
		</tr>
	)
}
