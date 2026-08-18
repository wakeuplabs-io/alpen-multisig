import { AccessibleDialog } from '@/components/accessible-dialog'
import { CopyClipboardIcon, CheckEmeraldIcon, AlertTriangleIcon } from '@/assets/icons'
import { useClipboardCopy } from '@/hooks/use-clipboard-copy'
import { useAdminIdCertificate } from '../hooks/use-admin-id-certificate'
import {
	CERTIFICATE_TITLE,
	CERTIFICATE_STEP_1_HEADING,
	CERTIFICATE_STEP_1_HELP,
	CERTIFICATE_WAITING,
	CERTIFICATE_SIGN_BUTTON,
	CERTIFICATE_SIGNED_CHIP,
	CERTIFICATE_COPIED,
	CERTIFICATE_STEP_2_HEADING,
	CERTIFICATE_STEP_2_HELP,
	CERTIFICATE_STEP_2_NO_DEVICE,
	certificateBlock,
} from '../model/admin-id-certificate'
import { VerifyOnDeviceButton } from './verify-on-device-button'
import type { AdminIdVerifyContext } from '../model/hw-device'

export type AdminIdCertificateModalProps = {
	isOpen: boolean
	onClose: () => void
	/** The Admin ID this certificate attests to (PRD 06 §3.b.ii.2). */
	adminId: string | undefined
	/**
	 * Present only for hardware sessions. Absent means a mnemonic signer, and Step 2 is
	 * disabled with an explanation rather than hidden — there is no device screen to
	 * compare the Admin ID against.
	 */
	verify?: AdminIdVerifyContext
}

/**
 * Admin ID Verification Certificate modal (PRD 06 §3.c.i, §4.a).
 *
 * Signs the Admin ID with its own key and shows the certificate, so anyone holding it can
 * recover the compressed public key behind the Admin ID — which is what #409 asked for and
 * no hardware signer can do by rendering a raw key on its screen.
 *
 * The layout, and every literal in it, come from the three wireframes in
 * `docs/0-prd/assets/`. Presentation only: the certificate is built and verified in Rust.
 */
export function AdminIdCertificateModal({ isOpen, onClose, adminId, verify }: AdminIdCertificateModalProps) {
	const { message, state, sign } = useAdminIdCertificate(adminId, isOpen)
	const { copied, error: copyError, copy } = useClipboardCopy()

	const certificate = state.status === 'signed' ? state.certificate : null
	const displayedMessage = message ?? ''

	function handleCopy() {
		if (!certificate) return
		copy(certificateBlock(certificate.message, certificate.certificate))
	}

	return (
		<AccessibleDialog isOpen={isOpen} onClose={onClose} title={CERTIFICATE_TITLE}>
			<div data-testid="e2e-admin-id-certificate-modal">
				<h3 className="m-0 mt-4 text-body font-semibold text-[#111827]">{CERTIFICATE_STEP_1_HEADING}</h3>
				<p className="m-0 mt-2 text-mono-sm leading-[1.45] text-[#6b7280]">{CERTIFICATE_STEP_1_HELP}</p>

				<p
					className="m-0 mt-3 break-all rounded-lg border border-[#e5e7eb] bg-white px-3 py-2.5 font-mono text-label leading-[1.5] text-[#374151]"
					data-testid="e2e-admin-id-certificate-message"
				>
					{displayedMessage}
				</p>

				<div className="mt-3 flex items-start gap-2 rounded-lg border border-[#e5e7eb] bg-white px-3 py-2.5">
					<p
						className="m-0 min-w-0 flex-1 break-all font-mono text-label leading-[1.5] text-[#374151]"
						data-testid="e2e-admin-id-certificate-value"
					>
						{certificate ? certificate.certificate : CERTIFICATE_WAITING}
					</p>
					{certificate && (
						<button
							type="button"
							onClick={handleCopy}
							aria-label={copied ? CERTIFICATE_COPIED : 'Copy the Admin ID Verification Certificate'}
							title={copyError ?? undefined}
							data-testid="e2e-admin-id-certificate-copy"
							className="inline-flex shrink-0 items-center justify-center rounded-md p-1.5 text-[#9ca3af] transition hover:bg-[#f3f4f6] hover:text-[#6b7280]"
						>
							{copied ? <CheckEmeraldIcon width={14} height={14} /> : <CopyClipboardIcon width={14} height={14} />}
						</button>
					)}
				</div>

				<div className="mt-3 flex items-center justify-between gap-3">
					{certificate ? (
						<span
							className="inline-flex items-center gap-1.5 rounded-md border border-[#6ee7b7] bg-[#ecfdf5] px-2.5 py-1 text-label font-medium text-[#065f46]"
							data-testid="e2e-admin-id-certificate-signed-chip"
						>
							<CheckEmeraldIcon width={13} height={13} />
							{CERTIFICATE_SIGNED_CHIP}
						</span>
					) : (
						<button
							type="button"
							onClick={() => void sign()}
							disabled={state.status === 'waiting' || !message}
							data-testid="e2e-admin-id-certificate-sign"
							className={`inline-flex items-center rounded-md px-4 py-1.5 text-label font-medium text-white transition ${
								state.status === 'waiting' || !message ? 'cursor-wait bg-[#9ca3af]' : 'bg-[#111827] hover:bg-[#374151]'
							}`}
						>
							{CERTIFICATE_SIGN_BUTTON}
						</button>
					)}
					{copied && (
						<span aria-live="polite" className="text-mono-sm text-[#6b7280]">
							{CERTIFICATE_COPIED}
						</span>
					)}
				</div>

				{copyError !== null && <p className="m-0 mt-2 text-mono-sm leading-[1.45] text-danger-strong">{copyError}</p>}

				<h3 className="m-0 mt-5 text-body font-semibold text-[#111827]">{CERTIFICATE_STEP_2_HEADING}</h3>
				<p className="m-0 mt-2 text-mono-sm leading-[1.45] text-[#6b7280]">{CERTIFICATE_STEP_2_HELP}</p>
				{verify ? (
					<VerifyOnDeviceButton
						deviceType={verify.deviceType}
						network={verify.network}
						derivationPath={verify.derivationPath}
						scriptType="p2wpkh"
						subject="Admin ID"
						expectedAddress={adminId ?? ''}
					/>
				) : (
					<p
						className="m-0 mt-2 text-mono-sm leading-[1.45] text-[#9ca3af]"
						data-testid="e2e-admin-id-certificate-no-device"
					>
						{CERTIFICATE_STEP_2_NO_DEVICE}
					</p>
				)}

				{state.status === 'error' && (
					<p
						aria-live="assertive"
						className="m-0 mt-3 inline-flex items-start gap-1.5 text-mono-sm leading-[1.45] text-danger-strong"
						data-testid="e2e-admin-id-certificate-error"
					>
						<AlertTriangleIcon width={13} height={13} className="mt-px shrink-0 text-danger" />
						<span>{state.message}</span>
					</p>
				)}
			</div>
		</AccessibleDialog>
	)
}
