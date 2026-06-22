import type { HwDeviceType } from '../model/hw-device'
import { CopyButton } from '@/components/copy-button'
import { ShieldCheckMutedIcon, AlertTriangleIcon } from '@/assets/icons'
import { isDisplayableAdminId, ADMIN_ID_LABEL, ADMIN_ID_SAFETY_CAPTION } from '../model/admin-id-presentation'
import { VerifyOnDeviceButton } from './verify-on-device-button'

/** Present only for HW sessions: drives the verify-on-device affordance (PRD §4.2). */
export type AdminIdVerifyContext = {
	deviceType: HwDeviceType
	network: string
	/**
	 * Connect-returned Admin ID path (BIP-84). Verifying against this exact path keeps the
	 * device showing the same key/coin it derived at connect — Trezor uses coin type 0',
	 * Ledger 1' on test nets — so app and device match (and the Trezor emulator, which
	 * rejects m/84'/1'/73', stays happy).
	 */
	derivationPath: string
}

export type AdminIdRowProps = {
	/** Canonical BIP-84 auth address (wallet.addressSample), or undefined when unknown. */
	adminId: string | undefined
	/** When set, renders a "Verify on device" affordance for the Admin ID (P2WPKH). */
	verify?: AdminIdVerifyContext
}

/**
 * Admin ID card (PRD §4.1): shows the signer's authentication identity in full
 * so it can be visually verified, with copy-to-clipboard. Styled as identity —
 * NOT a fundable address — and carries an explicit "do not send funds" caption,
 * since the Admin ID must never receive BTC or sign transactions.
 */
export function AdminIdRow({ adminId, verify }: AdminIdRowProps) {
	const label = (
		<span className="inline-flex items-center gap-1.5 text-mono-sm font-medium uppercase tracking-[0.08em] text-accent">
			<ShieldCheckMutedIcon width={13} height={13} className="text-accent" />
			{ADMIN_ID_LABEL}
		</span>
	)

	if (!isDisplayableAdminId(adminId)) {
		return (
			<div
				className="rounded-xl border border-accent-border bg-bg-surface px-4 py-3"
				data-testid="e2e-wallet-admin-id-row"
			>
				{label}
				<p className="mt-1.5 text-label text-[#9ca3af]">Unknown</p>
			</div>
		)
	}

	const value = adminId as string

	return (
		<div
			className="rounded-xl border border-accent-border bg-bg-surface px-4 py-3"
			data-testid="e2e-wallet-admin-id-row"
		>
			<div className="flex items-center justify-between gap-2">
				{label}
				<CopyButton text={value} variant="labeled" />
			</div>
			<p
				className="mt-1.5 break-all font-mono text-label leading-[1.5] text-[#374151]"
				title={value}
				data-testid="e2e-wallet-admin-id-value"
			>
				{value}
			</p>
			<p className="mt-2 inline-flex items-start gap-1.5 text-mono-sm leading-[1.45] text-[#b45309]">
				<AlertTriangleIcon width={13} height={13} className="mt-px shrink-0 text-[#d97706]" />
				<span>{ADMIN_ID_SAFETY_CAPTION}</span>
			</p>
			{verify && (
				<VerifyOnDeviceButton
					deviceType={verify.deviceType}
					network={verify.network}
					derivationPath={verify.derivationPath}
					scriptType="p2wpkh"
					subject="Admin ID"
				/>
			)}
		</div>
	)
}
