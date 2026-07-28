import type { HwDeviceType } from '../model/hw-device'
import { CopyButton } from '@/components/copy-button'
import { ShieldCheckMutedIcon, AlertTriangleIcon } from '@/assets/icons'
import { isDisplayableAdminId, ADMIN_ID_LABEL, ADMIN_ID_SAFETY_CAPTION } from '../model/admin-id-presentation'
import { adminIdVerifyCaption } from '@/lib/admin-id'
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
	/**
	 * Address the device will render for this key and path — the only thing a hardware
	 * signer can display (#409). Shown next to the key so the comparison is possible at all.
	 */
	address: string | undefined
}

export type AdminIdRowProps = {
	/** The Admin ID: the signer's compressed public key (#408), or undefined when unknown. */
	adminId: string | undefined
	/** When set, renders a "Verify on device" affordance for the Admin ID (P2WPKH). */
	verify?: AdminIdVerifyContext
}

/**
 * Admin ID card (PRD §4.1, corrected by issue #408): shows the signer's
 * authentication identity — the compressed public key — in full so it can be
 * visually verified, with copy-to-clipboard. It is an identity, NOT an address:
 * it must never receive BTC or sign Bitcoin transactions.
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
			<p className="mt-2 inline-flex items-start gap-1.5 text-mono-sm leading-[1.45] text-emphasis-soft">
				<AlertTriangleIcon width={13} height={13} className="mt-px shrink-0 text-emphasis-soft" />
				<span>{ADMIN_ID_SAFETY_CAPTION}</span>
			</p>
			{verify && (
				<>
					{verify.address && (
						<div className="mt-3 rounded-lg border border-[#f3f4f6] bg-[#fafafa] px-3 py-2">
							<p className="text-mono-sm font-medium uppercase tracking-[0.08em] text-[#9ca3af]">Address on device</p>
							<p
								className="mt-1 break-all font-mono text-label leading-[1.45] text-[#374151]"
								data-testid="e2e-wallet-admin-id-verify-address"
							>
								{verify.address}
							</p>
							<p className="mt-1 text-mono-sm leading-[1.45] text-[#9ca3af]">
								{adminIdVerifyCaption(verify.deviceType)} Compare this address, character for character, with the one on
								the device screen.
							</p>
						</div>
					)}
					<VerifyOnDeviceButton
						deviceType={verify.deviceType}
						network={verify.network}
						derivationPath={verify.derivationPath}
						scriptType="p2wpkh"
						subject="Admin ID"
						expectedAddress={verify.address}
					/>
				</>
			)}
		</div>
	)
}
