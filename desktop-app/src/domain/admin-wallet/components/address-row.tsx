import { CopyButton } from '@/components/copy-button'
import type { KeychainDto } from '@/domain/admin-wallet/model/types'
import { truncAddress } from '@/domain/admin-wallet/model/trunc-address'
import { formatBtcFromSats } from '@/domain/admin-wallet/model/format-btc-from-sats'
import { formatUnconfirmedBalanceLine } from '@/domain/admin-wallet/model/format-unconfirmed-balance-line'

export type AddressRowProps = {
	index: number
	address: string
	keychain: KeychainDto
	confirmedSats: number
	unconfirmedSats: number
	isUsed: boolean
}

export function AddressRow({
	index: _index,
	address,
	keychain,
	confirmedSats,
	unconfirmedSats,
	isUsed,
}: AddressRowProps) {
	const unconfirmedLine = formatUnconfirmedBalanceLine(unconfirmedSats)

	return (
		<tr
			className={`group transition-colors hover:bg-[#fafafa] ${isUsed ? 'text-[#111827]' : 'text-[#9ca3af]'}`}
			data-testid={`e2e-wallet-address-row-${address}`}
		>
			<td className="px-3 py-2 font-mono text-label" title={address}>
				<div className="flex items-center gap-1.5">
					<span className="min-w-0 truncate">{truncAddress(address)}</span>
					{keychain === 'Internal' && (
						<span
							className="flex-none rounded-full bg-[#f3f4f6] px-1.5 py-0.5 font-sans text-[10px] font-medium uppercase tracking-[0.04em] text-[#6b7280]"
							title="Change address (internal keychain)"
							data-testid="e2e-wallet-address-change-tag"
						>
							Change
						</span>
					)}
					<span className="opacity-60 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
						<CopyButton text={address} variant="icon" />
					</span>
				</div>
			</td>
			<td className="px-3 py-2 text-right font-mono text-body-sm tabular-nums">
				<div>{formatBtcFromSats(confirmedSats)} BTC</div>
				{unconfirmedLine !== null && (
					<div className="mt-0.5 text-mono-sm font-normal text-[#9ca3af]">{unconfirmedLine}</div>
				)}
			</td>
		</tr>
	)
}
