import { useState } from 'react'
import { CopyButton } from '@/components/copy-button'
import { ADMIN_ID_LABEL, ADMIN_ID_SAFETY_CAPTION, adminIdDisplay, isDisplayableAdminId } from '@/lib/admin-id'
import { AdminIdCertificateModal } from '@/domain/admin-wallet/components/admin-id-certificate-modal'
import type { AdminIdVerifyContext } from '@/domain/admin-wallet/model/hw-device'
import type { SessionAdminId } from '@/domain/admin-wallet/hooks/use-session-admin-id'

export type ConnectAdminIdCardProps = {
	/** The Admin ID derived at connect (PRD 06 §3.b.ii.2). The card shows its chain rendering. */
	adminId: SessionAdminId
	/** Present for hardware sessions, so the certificate modal can offer Step 2. */
	verify?: AdminIdVerifyContext
}

/**
 * Shows the signer their Admin ID *before* the app checks whether it belongs to the
 * canonical signer set (feedback 2026-07-01 #410). The user must be able to verify the
 * identity the app derived before the app passes judgement on it, so this card renders
 * while the membership check is still running.
 */
export function ConnectAdminIdCard({ adminId, verify }: ConnectAdminIdCardProps) {
	const shown = adminId.display
	const [isCertificateOpen, setIsCertificateOpen] = useState(false)

	return (
		<div className="rounded-xl border border-[#e5e7eb] bg-white px-5 py-4" data-testid="e2e-connect-admin-id-card">
			<div className="flex items-center justify-between gap-2">
				<p className="m-0 text-[0.65rem] font-medium uppercase tracking-[0.14em] text-[#9ca3af]">
					Your {ADMIN_ID_LABEL}
				</p>
				{isDisplayableAdminId(shown) && (
					<div className="flex items-center gap-1.5">
						<button
							type="button"
							onClick={() => setIsCertificateOpen(true)}
							data-testid="e2e-connect-admin-id-verify"
							className="inline-flex shrink-0 items-center rounded-md border border-[#e5e7eb] bg-white px-2.5 py-1.5 text-label font-medium text-[#6b7280] transition hover:border-[#d1d5db] hover:text-[#111827]"
						>
							Verify
						</button>
						<CopyButton text={shown} variant="labeled" />
					</div>
				)}
			</div>
			<p
				className="m-0 mt-2 break-all font-mono text-label leading-[1.5] text-[#111827]"
				data-testid="e2e-connect-admin-id-value"
			>
				{adminIdDisplay(shown, 'full')}
			</p>
			<p className="m-0 mt-2 text-mono-sm leading-[1.45] text-[#6b7280]">{ADMIN_ID_SAFETY_CAPTION}</p>
			<AdminIdCertificateModal
				isOpen={isCertificateOpen}
				onClose={() => setIsCertificateOpen(false)}
				adminId={verify ? adminId.device : shown}
				shownAdminId={shown}
				verify={verify}
			/>
		</div>
	)
}
