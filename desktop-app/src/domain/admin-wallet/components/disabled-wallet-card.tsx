import type { AdminWalletError } from '@/domain/admin-wallet/model/types'

type DisabledWalletCardProps = {
	error: AdminWalletError & ({ type: 'Disabled' } | { type: 'RegtestGuardViolation' })
}

const DISABLED_MESSAGE =
	'Admin Wallet is not enabled for this environment. Set BITCOIN_NETWORK=regtest and ALLOW_DEV_MNEMONIC_SIGNING=1 to enable.'

export function DisabledWalletCard({ error }: DisabledWalletCardProps) {
	const message = error.type === 'Disabled' ? DISABLED_MESSAGE : error.message

	return (
		<div className="rounded-xl border border-[#bfdbfe] bg-[#eff6ff] p-4">
			<div className="flex items-start gap-3">
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="#3b82f6"
					strokeWidth="2"
					strokeLinecap="round"
					strokeLinejoin="round"
					className="mt-0.5 shrink-0"
					aria-hidden="true"
				>
					<circle cx="12" cy="12" r="10" />
					<line x1="12" y1="8" x2="12" y2="12" />
					<line x1="12" y1="16" x2="12.01" y2="16" />
				</svg>
				<p className="m-0 text-[13px] leading-relaxed text-[#1e40af]">{message}</p>
			</div>
		</div>
	)
}
