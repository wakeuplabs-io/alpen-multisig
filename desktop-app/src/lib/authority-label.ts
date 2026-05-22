import { AuthRole } from '@/types/auth-role'

export function authorityLabelForRole(role: AuthRole): string {
	switch (role) {
		case AuthRole.StrataAdministrator:
			return 'Strata Administrator'
		case AuthRole.StrataSequencerManager:
			return 'Strata Sequencer Manager'
		case AuthRole.AlpenAdministrator:
			return 'Alpen Administrator'
		default:
			return 'Unknown authority'
	}
}
