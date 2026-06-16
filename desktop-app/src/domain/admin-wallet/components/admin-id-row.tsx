import { CopyButton } from '@/components/copy-button'
import { ShieldCheckMutedIcon, AlertTriangleIcon } from '@/assets/icons'
import { isDisplayableAdminId, ADMIN_ID_LABEL, ADMIN_ID_SAFETY_CAPTION } from '../model/admin-id-presentation'

export type AdminIdRowProps = {
	/** Canonical BIP-84 auth address (wallet.addressSample), or undefined when unknown. */
	adminId: string | undefined
}

/**
 * Admin ID card (PRD §4.1): shows the signer's authentication identity in full
 * so it can be visually verified, with copy-to-clipboard. Styled as identity —
 * NOT a fundable address — and carries an explicit "do not send funds" caption,
 * since the Admin ID must never receive BTC or sign transactions.
 */
export function AdminIdRow({ adminId }: AdminIdRowProps) {
	const label = (
		<span className="inline-flex items-center gap-1.5 text-[11px] font-medium uppercase tracking-[0.08em] text-[#7c6cf0]">
			<ShieldCheckMutedIcon width={13} height={13} className="text-[#9480f5]" />
			{ADMIN_ID_LABEL}
		</span>
	)

	if (!isDisplayableAdminId(adminId)) {
		return (
			<div className="rounded-xl border border-[#ece9fb] bg-[#faf9ff] px-4 py-3" data-testid="e2e-wallet-admin-id-row">
				{label}
				<p className="mt-1.5 text-[12px] text-[#9ca3af]">Unknown</p>
			</div>
		)
	}

	const value = adminId as string

	return (
		<div className="rounded-xl border border-[#ece9fb] bg-[#faf9ff] px-4 py-3" data-testid="e2e-wallet-admin-id-row">
			<div className="flex items-center justify-between gap-2">
				{label}
				<CopyButton text={value} variant="labeled" />
			</div>
			<p
				className="mt-1.5 break-all font-mono text-[12px] leading-[1.5] text-[#374151]"
				title={value}
				data-testid="e2e-wallet-admin-id-value"
			>
				{value}
			</p>
			<p className="mt-2 inline-flex items-start gap-1.5 text-[11px] leading-[1.45] text-[#b45309]">
				<AlertTriangleIcon width={13} height={13} className="mt-px shrink-0 text-[#d97706]" />
				<span>{ADMIN_ID_SAFETY_CAPTION}</span>
			</p>
		</div>
	)
}
