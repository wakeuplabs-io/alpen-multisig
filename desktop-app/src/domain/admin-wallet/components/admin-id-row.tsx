import { useState } from 'react'
import type { AdminIdVerifyContext } from '../model/hw-device'
import type { SessionAdminId } from '../hooks/use-session-admin-id'
import { CopyButton } from '@/components/copy-button'
import { ShieldCheckMutedIcon, AlertTriangleIcon } from '@/assets/icons'
import { isDisplayableAdminId, ADMIN_ID_LABEL, ADMIN_ID_SAFETY_CAPTION } from '../model/admin-id-presentation'
import { adminIdDisplay } from '@/lib/admin-id'
import { AdminIdCertificateModal } from './admin-id-certificate-modal'

export type AdminIdRowProps = {
	/** The session's Admin ID (PRD 06 §3.b.ii.2). */
	adminId: SessionAdminId
	/** When set, renders a "Verify on device" affordance for the Admin ID (P2WPKH). */
	verify?: AdminIdVerifyContext
}

/**
 * Admin ID card (PRD 06 §4.a): shows the signer's authentication identity in full so it
 * can be visually verified, with copy-to-clipboard, and warns that it must never receive
 * funds. The row shows and copies the chain rendering, the one every signer list and the
 * add-signer field use. On a hardware session the certificate modal gets the device's
 * rendering instead, because its message and Verify step are what the signer compares
 * against the device screen.
 */
export function AdminIdRow({ adminId, verify }: AdminIdRowProps) {
	const [isCertificateOpen, setIsCertificateOpen] = useState(false)
	const label = (
		<span className="inline-flex items-center gap-1.5 text-mono-sm font-medium uppercase tracking-[0.08em] text-emphasis">
			<ShieldCheckMutedIcon width={13} height={13} className="text-emphasis-soft" />
			{ADMIN_ID_LABEL}
		</span>
	)

	const value = adminId.display
	if (!isDisplayableAdminId(value)) {
		return (
			<div
				className="rounded-xl border border-accent-border bg-bg-surface px-4 py-3"
				data-testid="e2e-wallet-admin-id-row"
			>
				{label}
				<p className="mt-1.5 text-label text-[#9ca3af]">{adminIdDisplay(value, 'full')}</p>
			</div>
		)
	}

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
				adminId={verify ? adminId.device : value}
				shownAdminId={value}
				verify={verify}
			/>
		</div>
	)
}
