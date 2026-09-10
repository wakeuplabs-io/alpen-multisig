import { DEFCON_COPY, matchesDefconConfirmation, type DefconLevel } from '@/lib/defcon-copy'
import type { ActionValidator } from './types'

export function defconConfirmationError(level: DefconLevel): string {
	return `Type must match '${DEFCON_COPY[level].confirmation}' exactly (case-insensitive).`
}

/**
 * One gate, two levels. The registry stays keyed by action type — that exhaustiveness is what
 * makes a new action type a compile error rather than a silent no-op — but the rule itself is
 * written once, so the two levels cannot drift into accepting each other's confirmation string.
 */
export function validateDefcon(level: DefconLevel): ActionValidator {
	return ({ data, ctx }) => {
		// Signing something the form could not render is the one failure this action cannot afford,
		// so the resolved message gates submission exactly like the typed confirmation does.
		if (data.defconMessage.trim().length === 0) {
			ctx.addIssue({
				code: 'custom',
				path: ['defconMessage'],
				message: 'The signing message has not resolved yet.',
			})
		}
		if (!matchesDefconConfirmation(level, data.defconConfirm)) {
			ctx.addIssue({ code: 'custom', path: ['defconConfirm'], message: defconConfirmationError(level) })
		}
	}
}
