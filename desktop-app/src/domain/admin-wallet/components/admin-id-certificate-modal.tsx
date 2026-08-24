import { useEffect, useId, useRef } from 'react'
import { AccessibleDialog } from '@/components/accessible-dialog'
import { Button } from '@/components/button'
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
	CERTIFICATE_VERIFY_BUTTON,
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

const BOX_CLASSNAME = 'rounded-lg border px-3 py-2.5 font-mono text-label leading-[1.5] text-[#374151]'

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
	const { message, state, sign, reset } = useAdminIdCertificate(adminId, isOpen)
	const { copied, error: copyError, copy } = useClipboardCopy()
	const copyButtonRef = useRef<HTMLButtonElement>(null)
	const step1Id = useId()
	const step2Id = useId()
	const noDeviceId = useId()

	const certificate = state.status === 'signed' ? state.certificate : null
	const isWaiting = state.status === 'waiting'
	const isReady = message !== null

	// The Sign button unmounts the moment the certificate arrives, taking the focus with
	// it. Hand focus to the copy button that replaces it in the flow — otherwise focus
	// falls to <body> and the next Tab walks out of the dialog.
	useEffect(() => {
		if (certificate) copyButtonRef.current?.focus()
	}, [certificate])

	// A stale error must not greet the signer on reopen: "No Admin ID to sign yet." from a
	// previous attempt reads as a fresh failure of an action they have not taken.
	useEffect(() => {
		if (!isOpen) reset()
	}, [isOpen, reset])

	function handleCopy() {
		if (!certificate) return
		copy(certificateBlock(certificate.message, certificate.certificate))
	}

	// Closing mid-signature would leave the signer confirming on a device with nothing on
	// screen to receive it. The dialog says why it is busy instead of vanishing.
	function handleClose() {
		if (!isWaiting) onClose()
	}

	const politeAnnouncement = certificate
		? `${CERTIFICATE_SIGNED_CHIP}.${copied ? ` ${CERTIFICATE_COPIED}.` : ''}`
		: isWaiting
			? CERTIFICATE_WAITING
			: ''

	return (
		<AccessibleDialog
			isOpen={isOpen}
			onClose={handleClose}
			title={CERTIFICATE_TITLE}
			closeLabel="Close the Admin ID Verification Certificate"
			titleClassName="text-center"
			panelClassName="relative flex max-h-[85vh] w-full max-w-lg flex-col overflow-y-auto rounded-xl border border-[#e5e7eb] bg-white p-6 shadow-lg"
		>
			<div data-testid="e2e-admin-id-certificate-modal">
				{/* Live regions stay mounted for the life of the dialog: a region inserted in
				    the same tick as its text is routinely not announced. */}
				<div aria-live="polite" className="sr-only">
					{politeAnnouncement}
				</div>
				<div aria-live="assertive" className="sr-only">
					{state.status === 'error' ? state.message : (copyError ?? '')}
				</div>

				<section aria-labelledby={step1Id}>
					<h3 id={step1Id} className="m-0 mt-4 text-body font-semibold text-[#111827]">
						{CERTIFICATE_STEP_1_HEADING}
					</h3>
					<p className="m-0 mt-2 text-mono-sm leading-[1.45] text-[#6b7280]">{CERTIFICATE_STEP_1_HELP}</p>

					<p
						aria-label="Admin ID being signed"
						className={`m-0 mt-3 break-all border-[#e5e7eb] bg-white ${BOX_CLASSNAME}`}
						data-testid="e2e-admin-id-certificate-message"
					>
						{message ?? ''}
					</p>

					<div
						aria-busy={isWaiting}
						className={`mt-3 flex items-start gap-2 ${BOX_CLASSNAME} ${
							certificate ? 'border-[#6ee7b7] bg-[#ecfdf5]' : 'border-[#e5e7eb] bg-white'
						} ${isWaiting ? 'animate-pulse' : ''}`}
					>
						<p
							aria-label="Admin ID Verification Certificate"
							className="m-0 min-w-0 flex-1 break-all"
							data-testid="e2e-admin-id-certificate-value"
						>
							{certificate ? certificate.certificate : CERTIFICATE_WAITING}
						</p>
						{certificate && (
							<button
								ref={copyButtonRef}
								type="button"
								onClick={handleCopy}
								aria-label="Copy the Admin ID Verification Certificate"
								data-testid="e2e-admin-id-certificate-copy"
								className="inline-flex shrink-0 items-center justify-center rounded-md p-1.5 text-[#6b7280] transition hover:bg-[#f3f4f6] hover:text-[#111827]"
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
							<Button
								size="sm"
								onClick={() => void sign()}
								disabled={!isReady}
								loading={isWaiting}
								aria-busy={isWaiting}
								data-testid="e2e-admin-id-certificate-sign"
							>
								{CERTIFICATE_SIGN_BUTTON}
							</Button>
						)}
						{copied && <span className="text-mono-sm text-[#6b7280]">{CERTIFICATE_COPIED}</span>}
					</div>

					{/* Failures belong next to the control that caused them, not below the step
					    after this one, where a signer staring at a dead button never sees them. */}
					{state.status === 'error' && (
						<p
							className="m-0 mt-3 inline-flex items-start gap-1.5 text-mono-sm leading-[1.45] text-danger-strong"
							data-testid="e2e-admin-id-certificate-error"
						>
							<AlertTriangleIcon width={13} height={13} className="mt-px shrink-0 text-danger" />
							<span>{state.message}</span>
						</p>
					)}
					{copyError !== null && <p className="m-0 mt-2 text-mono-sm leading-[1.45] text-danger-strong">{copyError}</p>}
				</section>

				<section aria-labelledby={step2Id}>
					<h3 id={step2Id} className="m-0 mt-5 text-body font-semibold text-[#111827]">
						{CERTIFICATE_STEP_2_HEADING}
					</h3>
					<p className="m-0 mt-2 text-mono-sm leading-[1.45] text-[#6b7280]">{CERTIFICATE_STEP_2_HELP}</p>
					{verify ? (
						<VerifyOnDeviceButton
							deviceType={verify.deviceType}
							network={verify.network}
							derivationPath={verify.derivationPath}
							scriptType="p2wpkh"
							subject="Admin ID"
							expectedAddress={adminId ?? ''}
							label={CERTIFICATE_VERIFY_BUTTON}
							variant="primary"
						/>
					) : (
						<>
							{/* Disabled with the reason, never hidden: a step that disappears in one
							    session type reads as a broken modal rather than as not applicable. */}
							<Button
								size="sm"
								className="mt-2"
								disabled
								aria-describedby={noDeviceId}
								data-testid="e2e-admin-id-certificate-verify"
							>
								{CERTIFICATE_VERIFY_BUTTON}
							</Button>
							<p
								id={noDeviceId}
								className="m-0 mt-2 text-mono-sm leading-[1.45] text-[#6b7280]"
								data-testid="e2e-admin-id-certificate-no-device"
							>
								{CERTIFICATE_STEP_2_NO_DEVICE}
							</p>
						</>
					)}
				</section>
			</div>
		</AccessibleDialog>
	)
}
