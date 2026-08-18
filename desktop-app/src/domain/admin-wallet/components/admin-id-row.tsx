import { useState } from 'react'
import type { AdminIdVerifyContext } from '../model/hw-device'
import { CopyButton } from '@/components/copy-button'
import { ShieldCheckMutedIcon, AlertTriangleIcon } from '@/assets/icons'
import { isDisplayableAdminId, ADMIN_ID_LABEL, ADMIN_ID_SAFETY_CAPTION } from '../model/admin-id-presentation'
import { AdminIdCertificateModal } from './admin-id-certificate-modal'

export type AdminIdRowProps = {
	/** The Admin ID (PRD 06 §3.b.ii.2), or undefined when unknown. */
	adminId: string | undefined
	/** When set, renders a "Verify on device" affordance for the Admin ID (P2WPKH). */
	verify?: AdminIdVerifyContext
}

/**
 * Admin ID card (PRD 06 §4.a): shows the signer's authentication identity in full so it
 * can be visually verified, with copy-to-clipboard, and warns that it must never receive
 * funds. The value shown is the one handed to the device for verification, so the signer
 * compares the device screen against this exact string and nothing derived from it.
 */
export function AdminIdRow({ adminId, verify }: AdminIdRowProps) {
	const [isCertificateOpen, setIsCertificateOpen] = useState(false)
	const label = (
		<span className="inline-flex items-center gap-1.5 text-mono-sm font-medium uppercase tracking-[0.08em] text-emphasis">
			<ShieldCheckMutedIcon width={13} height={13} className="text-emphasis-soft" />
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

	const value = adminId

	return (
		<div
			className="rounded-xl border border-accent-border bg-bg-surface px-4 py-3"
			data-testid="e2e-wallet-admin-id-row"
		>
			<div className="flex items-center justify-between gap-2">
				{label}
				<div className="flex items-center gap-1.5">
					<button
						type="button"
						onClick={() => setIsCertificateOpen(true)}
						data-testid="e2e-wallet-admin-id-verify"
						className="inline-flex shrink-0 items-center rounded-md border border-[#e5e7eb] bg-white px-2.5 py-1.5 text-label font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
					>
						Verify
					</button>
					<CopyButton text={value} variant="labeled" />
				</div>
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
			<AdminIdCertificateModal
				isOpen={isCertificateOpen}
				onClose={() => setIsCertificateOpen(false)}
				adminId={value}
				verify={verify}
			/>
		</div>
	)
}
