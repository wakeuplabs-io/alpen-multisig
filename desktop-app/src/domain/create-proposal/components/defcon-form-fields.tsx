import { useEffect } from 'react'
import { useFormContext, useWatch } from 'react-hook-form'
import { DefconCallout } from '@/components/defcon-callout'
import { SafeHarbourNote } from '@/components/safe-harbour-note'
import { useDeviceSigningMessage } from '@/hooks/use-device-signing-message'
import { useSafeHarbourActivated } from '@/hooks/use-safe-harbour-status'
import { DEFCON_COPY, type DefconLevel } from '@/lib/defcon-copy'
import { useDefconActionHex } from '../hooks/use-defcon-action-hex'
import type { CreateProposalFormValues } from '../model/create-proposal.schema'
import { fieldErrorClass, monoInputDangerClass } from '../model/create-proposal-form-styles'

/** `seqNo` is a free-text field; the signing message can only be resolved for a real number. */
function parseSeqNo(raw: string | undefined): number | null {
	const trimmed = (raw ?? '').trim()
	return /^\d+$/.test(trimmed) ? Number(trimmed) : null
}

/**
 * One form for both Defcon levers. They differ in three strings, all of them read from
 * `DEFCON_COPY`; everything else here is the safety-critical half — the action-hex resolve, the
 * canonical message, its mirror into a form value and the gate that depends on it — and a second
 * copy of that is a place where one gets fixed and the other does not.
 *
 * The caller mounts this with `key={level}` so a switch remounts rather than carrying state over.
 */
export function DefconFormFields({ level }: { level: DefconLevel }) {
	const {
		control,
		register,
		setValue,
		formState: { errors },
	} = useFormContext<CreateProposalFormValues>()

	const copy = DEFCON_COPY[level]
	const testIdPrefix = `e2e-${level.replace('_', '-')}`
	const confirmInputId = `${level.replace('_', '-')}-confirm`
	const messageLabelId = `${level.replace('_', '-')}-signing-message-label`

	const { actionHex, error: actionHexError } = useDefconActionHex(level)
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
			{safeHarbourActivated && <SafeHarbourNote>{copy.safeHarbourNote}</SafeHarbourNote>}

			<DefconCallout level={level} />

			<div>
				<p id={messageLabelId} className="m-0 text-body font-medium text-emphasis">
					Signing message
				</p>
				{actionHexError === null ? (
					<pre
						aria-labelledby={messageLabelId}
						className="m-0 mt-1.5 overflow-x-auto whitespace-pre rounded-lg border border-[#e5e7eb] bg-bg-surface px-3 py-2.5 font-mono text-body text-emphasis"
						data-testid={`${testIdPrefix}-signing-message`}
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
				<label htmlFor={confirmInputId} className="text-body font-medium text-emphasis">
					Type <span className="font-mono font-semibold text-danger-deep">{copy.confirmation}</span> to confirm
				</label>
				<input
					id={confirmInputId}
					type="text"
					className={monoInputDangerClass}
					{...register('defconConfirm')}
					data-testid={`${testIdPrefix}-confirm`}
					autoComplete="off"
					spellCheck={false}
					aria-invalid={errors.defconConfirm !== undefined}
					aria-describedby={errors.defconConfirm ? `${confirmInputId}-error` : undefined}
				/>
				{errors.defconConfirm?.message && (
					<p id={`${confirmInputId}-error`} role="alert" className={fieldErrorClass}>
						{errors.defconConfirm.message}
					</p>
				)}
			</div>
		</div>
	)
}
