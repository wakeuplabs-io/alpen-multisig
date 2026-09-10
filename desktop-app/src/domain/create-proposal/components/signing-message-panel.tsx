type Props = {
	/** The canonical message resolved from Rust, or null while there is nothing to show. */
	message: string | null
	/** Shown in place of the message when it has not resolved. Must read as "not yet", never as an error. */
	placeholder: string
	/**
	 * Set only when resolution failed for a reason the signer cannot fix by finishing what they were
	 * typing. An invalid input is not one of those: it belongs under the field that took it.
	 */
	error: string | null
	testId: string
	labelId: string
	/**
	 * Replaces the default note under the panel. The safe harbour form uses it to say that the
	 * destination appears on the device as a descriptor rather than as an address — the one thing a
	 * signer has to know to compare the two, and the reason the panel is there at all.
	 */
	hint?: string
}

/**
 * The block that shows a signer what their device will display.
 *
 * Shared by both forms that have one. They used to be two copies of the same markup and the same
 * copy, which is how the safe harbour form inherited a sentence written for Defcon — "Reconnect and
 * try again" — and showed it under an address that was merely half typed.
 *
 * The rule the shape encodes: **a value that is not finished is not an error.** While nothing has
 * resolved, the panel reads as waiting. Red is reserved for a failure the signer cannot resolve by
 * continuing, and it deliberately offers no diagnosis it cannot make.
 */
export function SigningMessagePanel({ message, placeholder, error, testId, labelId, hint }: Props) {
	return (
		<div>
			<p id={labelId} className="m-0 text-body font-medium text-emphasis">
				Signing message
			</p>
			{error === null ? (
				<pre
					aria-labelledby={labelId}
					// Wraps instead of scrolling. Defcon's lines are short and never reached the edge;
					// the safe harbour message carries a 66-character descriptor that never fits, so
					// the one value a signer has to compare against their device was the one value
					// hidden past the right edge. `pre-wrap` keeps the line breaks and the indent that
					// make the message readable; `break-words` only breaks a token that cannot fit.
					className="m-0 mt-1.5 whitespace-pre-wrap break-words rounded-lg border border-[#e5e7eb] bg-bg-surface px-3 py-2.5 font-mono text-body text-emphasis"
					data-testid={testId}
				>
					{message ?? placeholder}
				</pre>
			) : (
				<p
					role="alert"
					className="mt-1.5 rounded-lg border border-danger-border bg-danger-surface px-3 py-2.5 text-body text-danger-deep"
				>
					{error}
				</p>
			)}
			<p className="mt-1 text-label text-emphasis-soft">
				{hint ?? 'This is exactly what you will see on your signer screen.'}
			</p>
		</div>
	)
}

/** The one sentence shown when the message genuinely could not be resolved. */
export const SIGNING_MESSAGE_UNRESOLVED =
	'The signing message could not be resolved, so there is nothing to compare against your signer.'
