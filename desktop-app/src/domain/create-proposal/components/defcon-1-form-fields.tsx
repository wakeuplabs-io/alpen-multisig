import { useEffect } from 'react'
import { useFormContext, useWatch } from 'react-hook-form'
import { AlertTriangleIcon } from '@/assets/icons'
import { SafeHarbourNote } from '@/components/safe-harbour-note'
import { useDeviceSigningMessage } from '@/hooks/use-device-signing-message'
import { useSafeHarbourActivated } from '@/hooks/use-safe-harbour-status'
import { useDefcon1ActionHex } from '../hooks/use-defcon-1-action-hex'
import type { CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, monoInputDangerClass } from '../model/create-proposal-form-styles'
import { DEFCON_1_CONFIRMATION } from '../model/validators/defcon-1'

const CONFIRM_INPUT_ID = 'defcon-1-confirm'
const MESSAGE_LABEL_ID = 'defcon-1-signing-message-label'

/** `seqNo` is a free-text field; the signing message can only be resolved for a real number. */
function parseSeqNo(raw: string | undefined): number | null {
	const trimmed = (raw ?? '').trim()
	return /^\d+$/.test(trimmed) ? Number(trimmed) : null
}

export function Defcon1FormFields() {
	const {
		control,
		register,
		setValue,
		formState: { errors },
	} = useFormContext<CreateProposalFormValues>()

	const { actionHex, error: actionHexError } = useDefcon1ActionHex()
	const safeHarbourActivated = useSafeHarbourActivated()
	const seqNo = parseSeqNo(useWatch({ control, name: 'seqNo' }))
	// Rendered, never written: the four canonical lines come from the same Rust renderer the
	// device signs over, so they cannot drift from what the signer is about to confirm.
	const { message } = useDeviceSigningMessage(seqNo, actionHex)

	// The resolved message is a form value, so "the signer can see what they are signing" is
	// part of the same validity the CTAs already gate on rather than a second, separate switch.
	useEffect(() => {
		setValue('defconMessage', message ?? '', { shouldValidate: true })
	}, [message, setValue])

	// Four states, kept distinct: a broken resolve must never read like a prompt.
	const placeholder = seqNo === null ? 'Enter a sequence number to resolve the signing message.' : 'Resolving…'

	return (
		<div className="flex flex-col gap-5">
			{/* Told, never enforced: the type-to-confirm gate below stays the only gate. */}
			{safeHarbourActivated && (
				<SafeHarbourNote>
					The bridge is already in safe harbour. Another Defcon 1 does not change that — it consumes a council sequence
					number, costs fees, and needs a full quorum. Create one only if you have reason to believe this state is
					wrong.
				</SafeHarbourNote>
			)}

			<div className="rounded-xl border border-danger-border bg-danger-surface p-4">
				<p className="m-0 flex items-center gap-2 text-body font-semibold text-danger-deep">
					<AlertTriangleIcon width={16} height={16} className="shrink-0 text-danger" />
					Irreversible
				</p>
				<p className="m-0 mt-2 text-body text-danger-deep">
					DEFCON 1 activates the Safe Harbor sweep immediately, taking effect in the block that the approved proposal is
					confirmed in. Once approved and confirmed, it cannot be canceled, and is therefore irreversible.
				</p>
			</div>

			<div>
				<p id={MESSAGE_LABEL_ID} className="m-0 text-body font-medium text-emphasis">
					Signing message
				</p>
				{actionHexError === null ? (
					<pre
						aria-labelledby={MESSAGE_LABEL_ID}
						className="m-0 mt-1.5 overflow-x-auto whitespace-pre rounded-lg border border-[#e5e7eb] bg-bg-surface px-3 py-2.5 font-mono text-body text-emphasis"
						data-testid="e2e-defcon-1-signing-message"
					>
						{message ?? placeholder}
					</pre>
				) : (
					<p
						role="alert"
						className="mt-1.5 rounded-lg border border-danger-border bg-danger-surface px-3 py-2.5 text-body text-danger-deep"
					>
						The signing message could not be resolved, so there is nothing to compare against your signer. Reconnect and
						try again. ({actionHexError})
					</p>
				)}
				{/* Only once the (constant) action hex is in hand: before that the box is still doing
				    its first fetch, and an error there would flash on every mount. */}
				{actionHex !== null && errors.defconMessage?.message ? (
					<p role="alert" className={fieldErrorClass}>
						{errors.defconMessage.message} Nothing can be signed until it does.
					</p>
				) : (
					<p className="mt-1 text-label text-emphasis-soft">This is exactly what you will see on your signer screen.</p>
				)}
			</div>

			<div>
				<label htmlFor={CONFIRM_INPUT_ID} className="text-body font-medium text-emphasis">
					Type <span className="font-mono font-semibold text-danger-deep">{DEFCON_1_CONFIRMATION}</span> to confirm
				</label>
				<input
					id={CONFIRM_INPUT_ID}
					type="text"
					className={monoInputDangerClass}
					{...register('defconConfirm')}
					data-testid="e2e-defcon-1-confirm"
					autoComplete="off"
					spellCheck={false}
					aria-invalid={errors.defconConfirm !== undefined}
					aria-describedby={errors.defconConfirm ? `${CONFIRM_INPUT_ID}-error` : undefined}
				/>
				{errors.defconConfirm?.message && (
					<p id={`${CONFIRM_INPUT_ID}-error`} role="alert" className={fieldErrorClass}>
						{errors.defconConfirm.message}
					</p>
				)}
			</div>
		</div>
	)
}
