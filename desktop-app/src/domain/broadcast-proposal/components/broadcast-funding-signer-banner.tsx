import type { WalletVendor } from '@/wallet/types'
import { deviceCopy } from '@/lib/device-copy'

type Props = {
	backendSignerKind: 'hardware' | 'mnemonic' | 'none'
	connectVendor: WalletVendor
}

function labelFor(kind: 'hardware' | 'mnemonic' | 'none', vendor: WalletVendor): string {
	if (kind === 'hardware') {
		return `${deviceCopy(vendor).label} (on-device)`
	}
	if (kind === 'mnemonic') {
		return 'Mnemonic / software (no device prompt for commit)'
	}
	return 'Not connected'
}

export function BroadcastFundingSignerBanner({ backendSignerKind, connectVendor }: Props) {
	const mismatch = (connectVendor === 'ledger' || connectVendor === 'trezor') && backendSignerKind !== 'hardware'

	return (
		<div
			className={[
				'rounded-lg border px-4 py-3 text-body-sm',
				mismatch
					? 'border-danger-border bg-danger-surface text-danger-deep'
					: 'border-[#e5e7eb] bg-[#f9fafb] text-[#374151]',
			].join(' ')}
		>
			<p className="m-0 font-medium text-[#111827]">
				Commit funding signer:{' '}
				<span data-testid="e2e-broadcast-funding-signer">{labelFor(backendSignerKind, connectVendor)}</span>
			</p>
			{backendSignerKind === 'hardware' ? (
				<p className="m-0 mt-1 text-[#6b7280]">{deviceCopy(connectVendor).broadcastHint}</p>
			) : backendSignerKind === 'mnemonic' ? (
				<p className="m-0 mt-1 text-[#6b7280]">
					The commit tx is signed in software (same seed as Mnemonic) — no device will ask for this step.
				</p>
			) : null}
			{mismatch && (
				<p className="m-0 mt-2 text-danger-deep">
					You connected {deviceCopy(connectVendor).label} in the UI, but the Admin Wallet session is not hardware-bound.
					Disconnect, choose <strong>{deviceCopy(connectVendor).label}</strong> (not Mnemonic), connect, authenticate
					again, then retry broadcast.
				</p>
			)}
		</div>
	)
}
