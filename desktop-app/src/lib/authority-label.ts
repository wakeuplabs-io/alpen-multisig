import { AuthRole } from '@/types/auth-role'

export function authorityLabelForRole(role: AuthRole): string {
	switch (role) {
		case AuthRole.StrataAdministrator:
			return 'Strata Administrator'
		case AuthRole.StrataSequencerManager:
			return 'Strata Sequencer Manager'
		case AuthRole.AlpenAdministrator:
			return 'Alpen Administrator'
		case AuthRole.StrataSecurityCouncil:
			// AC 14 pins this exact text. Upstream's own role name — the one the signing message
			// carries — is "Strata Security Council"; that string comes from the protocol and is
			// rendered verbatim beside this badge, never restated here.
			return 'Security Council'
		default:
			return 'Unknown authority'
	}
}
