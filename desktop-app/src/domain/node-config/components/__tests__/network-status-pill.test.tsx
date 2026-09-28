// NetworkStatusPill — the node mode indicator in the app header (#560).
//
// Before sign-in the pill is a button that opens node settings; after sign-in it must stay visible as a
// read-only indicator so the signer always knows which environment they are operating in.

import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { renderToStaticMarkup } from 'react-dom/server'

import { NetworkStatusPill } from '../network-status-pill.tsx'
import type { LocalNodeStatus } from '../../model/node-config.types.ts'

const REACHABLE: LocalNodeStatus = {
	strataReachable: true,
	btcReachable: true,
	electrumReachable: true,
	orchestratorReachable: true,
	strataUrl: '',
	btcUrl: '',
	electrumUrl: '',
	orchestratorUrl: '',
}
const UNREACHABLE: LocalNodeStatus = { ...REACHABLE, btcReachable: false }

// ── 1. Editable (pre-auth): a button that opens node settings ─────────────────
const editable = renderToStaticMarkup(
	<NetworkStatusPill mode="local" localNodeStatus={REACHABLE} onClick={() => undefined} />,
)
assert.ok(editable.startsWith('<button'), 'with onClick the pill must be a button')
assert.ok(editable.includes('Open node settings.'), 'the editable pill must say it opens node settings')
assert.ok(editable.includes('<svg'), 'the editable pill must show the settings gear')
console.log('NetworkStatusPill: editable variant OK')

// ── 2. Read-only (post-auth): same information, nothing to click ──────────────
const readOnly = renderToStaticMarkup(<NetworkStatusPill mode="trusted" localNodeStatus={REACHABLE} />)
assert.ok(!readOnly.includes('<button'), 'without onClick the pill must not be a button')
assert.ok(!readOnly.includes('Open node settings'), 'the read-only pill must not offer node settings')
assert.ok(!readOnly.includes('<svg'), 'the read-only pill must not show the settings gear')
assert.ok(readOnly.includes('role="status"'), 'the read-only pill must be announced as a status')
assert.ok(
	readOnly.includes('aria-label="Network: Trusted, connected"'),
	'read-only aria-label must name mode and state',
)
console.log('NetworkStatusPill: read-only variant OK')

// ── 3. Mode label and connectivity dot ───────────────────────────────────────
for (const [mode, label] of [
	['local', 'Local'],
	['trusted', 'Trusted'],
	['custom', 'Custom'],
] as const) {
	const html = renderToStaticMarkup(<NetworkStatusPill mode={mode} localNodeStatus={REACHABLE} />)
	assert.ok(html.includes(`>${label}<`), `mode ${mode} must render the ${label} label`)
}
assert.ok(readOnly.includes('bg-emerald-500'), 'a reachable node must show the green dot')
const issue = renderToStaticMarkup(<NetworkStatusPill mode="custom" localNodeStatus={UNREACHABLE} />)
assert.ok(!issue.includes('bg-emerald-500'), 'an unreachable service must not show the green dot')
assert.ok(issue.includes('connection issue'), 'an unreachable service must be announced as a connection issue')
const unknown = renderToStaticMarkup(<NetworkStatusPill mode="local" localNodeStatus={null} />)
assert.ok(unknown.includes('connection issue'), 'an unchecked node must not claim to be connected')
console.log('NetworkStatusPill: labels and connectivity dot OK')

// ── 4. Wiring: the shell renders it on every screen, editable only pre-auth ───
const __dirname = dirname(fileURLToPath(import.meta.url))
const screensDir = join(__dirname, '..', '..', '..', '..', 'screens')
const shellSource = readFileSync(join(screensDir, 'screen-shell.tsx'), 'utf8')
const connectSource = readFileSync(join(screensDir, 'wallet-connect-screen.tsx'), 'utf8')
assert.ok(shellSource.includes('<NetworkStatusPill'), 'ScreenShell must render the network status pill')
assert.ok(
	shellSource.includes('onClick={onOpenNodeSettings}'),
	'ScreenShell must make the pill editable only on request',
)
assert.ok(connectSource.includes('onOpenNodeSettings='), 'the connect screen must keep the pill editable')
assert.ok(!connectSource.includes('<NetworkStatusPill'), 'the connect screen must not render a second pill')
console.log('NetworkStatusPill: shell wiring OK')
