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

export function AddressRow({ index: _index, address, confirmedSats, unconfirmedSats, isUsed }: AddressRowProps) {
	const unconfirmedLine = formatUnconfirmedBalanceLine(unconfirmedSats)

	return (
		<tr className={`group transition-colors hover:bg-[#fafafa] ${isUsed ? 'text-[#111827]' : 'text-[#9ca3af]'}`}>
			<td className="px-3 py-2 font-mono text-[12px]" title={address}>
				<div className="flex items-center gap-1.5">
					<span className="min-w-0 truncate">{truncAddress(address)}</span>
					<span className="opacity-60 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
						<CopyButton text={address} />
					</span>
				</div>
			</td>
			<td className="px-3 py-2 text-right font-mono text-[13px] tabular-nums">
				<div>{formatBtcFromSats(confirmedSats)} BTC</div>
				{unconfirmedLine !== null && (
					<div className="mt-0.5 text-[11px] font-normal text-[#9ca3af]">{unconfirmedLine}</div>
				)}
			</td>
		</tr>
	)
}
