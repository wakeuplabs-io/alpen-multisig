import type { OrchestratorAuthority } from '@/api/orchestrator-auth'
import { AuthRole } from '@/types/auth-role'

// Display names for the authority ids the orchestrator stores on each proposal. The id itself
// (`strata_admin`, …) stays the wire value — signing messages, exports and API calls keep it.
const AUTHORITY_LABELS: Record<OrchestratorAuthority, string> = {
	strata_admin: 'Strata Administrator',
	sequencer_manager: 'Strata Sequencer Manager',
	alpen_admin: 'Alpen Administrator',
	// AC 14 pins this exact text. Upstream's own role name — the one the signing message
	// carries — is "Strata Security Council"; that string comes from the protocol and is
	// rendered verbatim beside this badge, never restated here.
	security_council: 'Security Council',
	payout_admin: 'Payout Administrator',
}

function isKnownAuthority(authority: string): authority is OrchestratorAuthority {
	return Object.prototype.hasOwnProperty.call(AUTHORITY_LABELS, authority)
}

// An id this build does not know is shown as-is: hiding what the backend sent would leave a
// signer unable to tell which authority a proposal belongs to.
export function authorityDisplayName(authority: string): string {
	return isKnownAuthority(authority) ? AUTHORITY_LABELS[authority] : authority
}

// Mirrors `authorityFromRole` in `@/api/orchestrator-auth`, which cannot be imported here at runtime:
// that module reads `import.meta.env` on load. The `Record` keeps this exhaustive at compile time.
const ROLE_AUTHORITIES: Record<AuthRole, OrchestratorAuthority> = {
	[AuthRole.StrataAdministrator]: 'strata_admin',
	[AuthRole.StrataSequencerManager]: 'sequencer_manager',
	[AuthRole.AlpenAdministrator]: 'alpen_admin',
	[AuthRole.StrataSecurityCouncil]: 'security_council',
	[AuthRole.PayoutAdministrator]: 'payout_admin',
}

export function authorityLabelForRole(role: AuthRole): string {
	return authorityDisplayName(ROLE_AUTHORITIES[role])
}
