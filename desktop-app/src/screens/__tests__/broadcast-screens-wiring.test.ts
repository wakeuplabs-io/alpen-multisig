// Both send screens must wire the Admin Wallet through the same shared hook.
//
// Issue #484: `cancel-proposal-broadcast-screen.tsx` never wired it, so Confirm & Send was
// disabled at every threshold and a queued update could not be cancelled from the UI. `tsc` now
// rejects an unwired card (the admin-wallet props are required), and this test guards the rest of
// the contract that types cannot express: that both screens go through
// `useBroadcastAdminWallet(adapter)` instead of re-assembling the hooks screen-side, and that the
// cancel flow forwards `signerKind` + `adapter` down to `useBroadcastProposal`.
//
// Source-text assertions, following the project's existing tsx-runner style — React rendering
// tests need vitest + @testing-library/react (BLOCKED_BY_DEPENDENCY — not installed).

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const screensDir = join(dirname(fileURLToPath(import.meta.url)), '..')

const screens = {
	'broadcast-proposal-screen.tsx': readFileSync(join(screensDir, 'broadcast-proposal-screen.tsx'), 'utf8'),
	'cancel-proposal-broadcast-screen.tsx': readFileSync(
		join(screensDir, 'cancel-proposal-broadcast-screen.tsx'),
		'utf8',
	),
}

for (const [name, source] of Object.entries(screens)) {
	assert.ok(source.includes('useBroadcastAdminWallet(adapter)'), `${name}: must use the shared admin-wallet hook`)
	assert.ok(source.includes('{...cardProps}'), `${name}: must spread the hook's cardProps into BroadcastDetailsCard`)
	assert.ok(source.includes('phase={phase}'), `${name}: must pass the phase (device prompt + button label)`)
	assert.ok(source.includes('useWalletPanelData(isAdminWalletMode)'), `${name}: panel must follow admin-wallet mode`)
	assert.ok(source.includes('BroadcastFundingSignerBanner'), `${name}: must warn on a signer/vendor mismatch`)

	// The predicates are shared so the two screens cannot drift again: the cancel screen's local
	// copy omitted awaiting-device and awaiting-confirmation, blanking the UI mid-confirmation.
	assert.ok(source.includes('isBroadcastProgressPhase(phase)'), `${name}: must use the shared progress predicate`)
	assert.ok(source.includes('isBroadcastDetailsPhase(phase)'), `${name}: must use the shared details predicate`)
	assert.ok(source.includes('isBroadcastInFlightPhase(phase)'), `${name}: must use the shared in-flight predicate`)

	// Re-declaring these screen-side is exactly the duplication the shared hook removed.
	for (const hook of ['useAdminWalletInfo', 'useAdminWalletCapability', 'useEnsureAdminWalletSession']) {
		assert.ok(!source.includes(`${hook}(`), `${name}: ${hook} belongs in useBroadcastAdminWallet, not the screen`)
	}
}

// The cancel flow reaches useBroadcastProposal through useCancelBroadcast, so the two extra
// arguments have to be forwarded at both hops — without them the Admin Wallet session is never
// re-bound and a hardware signer never gets its "Approve on device…" prompt.
const cancelScreen = screens['cancel-proposal-broadcast-screen.tsx']
assert.ok(
	/useCancelBroadcast\(.*?signerKind,\s*adapter\)/s.test(cancelScreen),
	'cancel screen must pass signerKind and adapter to useCancelBroadcast',
)

const cancelHook = readFileSync(
	join(screensDir, '..', 'domain', 'cancel-proposal', 'hooks', 'use-cancel-broadcast.ts'),
	'utf8',
)
assert.ok(
	/useBroadcastProposal\(.*?signerKind,\s*adapter\)/s.test(cancelHook),
	'useCancelBroadcast must forward signerKind and adapter to useBroadcastProposal',
)

console.log('broadcast screens: admin-wallet wiring parity OK')
