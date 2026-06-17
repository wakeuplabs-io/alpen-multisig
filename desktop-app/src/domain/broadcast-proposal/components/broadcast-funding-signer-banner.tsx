import type { WalletVendor } from '@/wallet/types'

type Props = {
	backendSignerKind: 'hardware' | 'mnemonic' | 'none'
	connectVendor: WalletVendor
}

function labelFor(kind: 'hardware' | 'mnemonic' | 'none', vendor: WalletVendor): string {
	if (kind === 'hardware') {
		return vendor === 'trezor' ? 'Trezor (on-device)' : 'Ledger (on-device)'
	}
	if (kind === 'mnemonic') {
		return 'Palabras / software (no device prompt for commit)'
	}
	return 'Not connected'
}

export function BroadcastFundingSignerBanner({ backendSignerKind, connectVendor }: Props) {
	const mismatch = (connectVendor === 'ledger' || connectVendor === 'trezor') && backendSignerKind !== 'hardware'

	return (
		<div
			className={[
				'rounded-lg border px-4 py-3 text-body-sm',
				mismatch ? 'border-[#fcd34d] bg-[#fffbeb] text-[#92400e]' : 'border-[#e5e7eb] bg-[#f9fafb] text-[#374151]',
			].join(' ')}
		>
			<p className="m-0 font-medium text-[#111827]">
				Commit funding signer:{' '}
				<span data-testid="e2e-broadcast-funding-signer">{labelFor(backendSignerKind, connectVendor)}</span>
			</p>
			{backendSignerKind === 'hardware' ? (
				<p className="m-0 mt-1 text-[#6b7280]">
					Broadcast is not a single “Sign message” like login: Ledger first registers the wallet policy, then shows the
					commit transaction. Approve <strong>every</strong> screen with <strong>right</strong> (policy + tx).
				</p>
			) : backendSignerKind === 'mnemonic' ? (
				<p className="m-0 mt-1 text-[#6b7280]">
					The commit tx is signed in software (same seed as Palabras). The Ledger emulator will not ask for this step.
				</p>
			) : null}
			{mismatch && (
				<p className="m-0 mt-2 text-[#b45309]">
					You connected {connectVendor} in the UI, but the Admin Wallet session is not hardware-bound. Disconnect,
					choose <strong>Ledger</strong> (not Palabras), connect, authenticate again, then retry broadcast.
				</p>
			)}
		</div>
	)
}
