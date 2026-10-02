// authority-label — the display name every proposal header shows instead of the raw authority id.
//
// Proposal detail, broadcast and cancel used to render `strata_admin` verbatim (#583). The id is
// still the wire value; only the rendered text changes, so each mapping is asserted exactly.

import assert from 'node:assert/strict'
import { authorityDisplayName, authorityLabelForRole } from '../authority-label.ts'
import { AuthRole } from '../../types/auth-role.ts'

assert.equal(authorityDisplayName('strata_admin'), 'Strata Administrator')
assert.equal(authorityDisplayName('alpen_admin'), 'Alpen Administrator')
assert.equal(authorityDisplayName('sequencer_manager'), 'Strata Sequencer Manager')
assert.equal(authorityDisplayName('security_council'), 'Security Council')
assert.equal(authorityDisplayName('payout_admin'), 'Payout Administrator')

// An id this build does not know is shown as-is, never swallowed into a generic label.
assert.equal(authorityDisplayName('future_authority'), 'future_authority')
assert.equal(authorityDisplayName('toString'), 'toString')

// The session badge and the proposal header must name the same authority the same way.
assert.equal(authorityLabelForRole(AuthRole.StrataAdministrator), 'Strata Administrator')
assert.equal(authorityLabelForRole(AuthRole.AlpenAdministrator), 'Alpen Administrator')
assert.equal(authorityLabelForRole(AuthRole.StrataSequencerManager), 'Strata Sequencer Manager')
assert.equal(authorityLabelForRole(AuthRole.StrataSecurityCouncil), 'Security Council')
assert.equal(authorityLabelForRole(AuthRole.PayoutAdministrator), 'Payout Administrator')

console.log('authority-label: ok')
