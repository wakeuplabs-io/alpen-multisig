// Architecture compliance test — dependency-direction rules for admin-wallet domain.
//
// Rules enforced:
//   1. domain/admin-wallet/components/ must NOT import from @tauri-apps/api/core or @/api/admin-wallet
//   2. domain/admin-wallet/model/ (production files) must NOT import 'react'
//   3. No production file under domain/admin-wallet/ may import from __fixtures__/

import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'

const domainRoot = new URL('.', import.meta.url).pathname

// ── Helpers ──────────────────────────────────────────────────────────────────

function collectFiles(dir: string, predicate: (f: string) => boolean): string[] {
	const results: string[] = []
	if (!fs.existsSync(dir)) return results
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const full = path.join(dir, entry.name)
		if (entry.isDirectory()) {
			results.push(...collectFiles(full, predicate))
		} else if (predicate(full)) {
			results.push(full)
		}
	}
	return results
}

function isTsFile(f: string): boolean {
	return f.endsWith('.ts') || f.endsWith('.tsx')
}

function isTestDir(f: string): boolean {
	return f.includes('__tests__') || f.includes('__fixtures__')
}

function isTestFile(f: string): boolean {
	return f.endsWith('.test.ts') || f.endsWith('.test.tsx')
}

// ── Rule 1: components/ must not import from @tauri-apps/api/core or @/api/admin-wallet ──

const componentsDir = path.join(domainRoot, 'components')
const componentFiles = collectFiles(componentsDir, isTsFile)

const forbiddenComponentImports = ['@tauri-apps/api/core', '@/api/admin-wallet']

const rule1Violations: string[] = []
for (const file of componentFiles) {
	const content = fs.readFileSync(file, 'utf8')
	for (const forbidden of forbiddenComponentImports) {
		if (content.includes(`from "${forbidden}"`) || content.includes(`from '${forbidden}'`)) {
			rule1Violations.push(`${path.relative(domainRoot, file)}: imports '${forbidden}'`)
		}
	}
}

assert.equal(
	rule1Violations.length,
	0,
	`Rule 1 violations — components must not import infrastructure:\n  ${rule1Violations.join('\n  ')}`,
)
console.log(`Rule 1 PASS: ${componentFiles.length} component file(s) checked — no forbidden imports`)

// ── Rule 2: model/ production files must not import 'react' ──────────────────

const modelDir = path.join(domainRoot, 'model')
const modelProductionFiles = collectFiles(modelDir, (f) => isTsFile(f) && !isTestDir(f))

const rule2Violations: string[] = []
for (const file of modelProductionFiles) {
	const content = fs.readFileSync(file, 'utf8')
	if (content.includes(`from "react"`) || content.includes(`from 'react'`) || content.includes('import React')) {
		rule2Violations.push(path.relative(domainRoot, file))
	}
}

assert.equal(
	rule2Violations.length,
	0,
	`Rule 2 violations — model files must not import react:\n  ${rule2Violations.join('\n  ')}`,
)
console.log(`Rule 2 PASS: ${modelProductionFiles.length} model file(s) checked — no react imports`)

// ── Rule 3: production files must not import from __fixtures__/ ──────────────

const allProductionFiles = collectFiles(domainRoot, (f) => isTsFile(f) && !isTestDir(f) && !isTestFile(f))

const rule3Violations: string[] = []
for (const file of allProductionFiles) {
	const content = fs.readFileSync(file, 'utf8')
	if (content.includes('__fixtures__/')) {
		rule3Violations.push(path.relative(domainRoot, file))
	}
}

assert.equal(
	rule3Violations.length,
	0,
	`Rule 3 violations — production files must not import from __fixtures__:\n  ${rule3Violations.join('\n  ')}`,
)
console.log(`Rule 3 PASS: ${allProductionFiles.length} production file(s) checked — no __fixtures__ imports`)

// ── Rule 4: no dev/roadmap placeholder copy leaks into the wallet panel (R1.2) ──

const roadmapCopyPatterns = [/arrives in Phase/i, /not available yet/i, /QR preview unavailable/i, /Admin tools/]

const rule4Violations: string[] = []
for (const file of componentFiles) {
	if (isTestFile(file)) continue
	const content = fs.readFileSync(file, 'utf8')
	for (const pattern of roadmapCopyPatterns) {
		if (pattern.test(content)) {
			rule4Violations.push(`${path.relative(domainRoot, file)}: matches ${pattern}`)
		}
	}
}

assert.equal(
	rule4Violations.length,
	0,
	`Rule 4 violations — wallet panel must not contain dev/roadmap placeholder copy:\n  ${rule4Violations.join('\n  ')}`,
)
console.log(`Rule 4 PASS: ${componentFiles.length} component file(s) checked — no roadmap placeholder copy`)

// ── Rule 5: R1.5 balance wiring — panel passes confirmed + unconfirmed to WalletBalance ──

const walletPanelContentPath = path.join(componentsDir, 'wallet-panel-content.tsx')
const walletBalancePath = path.join(componentsDir, 'wallet-balance.tsx')
const panelContent = fs.readFileSync(walletPanelContentPath, 'utf8')
const walletBalance = fs.readFileSync(walletBalancePath, 'utf8')

const rule5Violations: string[] = []
if (!panelContent.includes('confirmedBalanceSats')) {
	rule5Violations.push('wallet-panel-content.tsx: missing confirmedBalanceSats prop')
}
if (!panelContent.includes('unconfirmedBalanceSats')) {
	rule5Violations.push('wallet-panel-content.tsx: missing unconfirmedBalanceSats prop')
}
if (!panelContent.includes('confirmedSats={confirmedBalanceSats}')) {
	rule5Violations.push('wallet-panel-content.tsx: must forward confirmedSats to WalletBalance')
}
if (!panelContent.includes('unconfirmedSats={unconfirmedBalanceSats}')) {
	rule5Violations.push('wallet-panel-content.tsx: must forward unconfirmedSats to WalletBalance')
}
if (!walletBalance.includes('formatUnconfirmedBalanceLine')) {
	rule5Violations.push('wallet-balance.tsx: must use formatUnconfirmedBalanceLine for unconfirmed copy')
}
if (walletBalance.includes('balanceSats:')) {
	rule5Violations.push('wallet-balance.tsx: must not use legacy single balanceSats prop')
}

assert.equal(
	rule5Violations.length,
	0,
	`Rule 5 violations — R1.5 balance UX wiring:\n  ${rule5Violations.join('\n  ')}`,
)
console.log('Rule 5 PASS: wallet panel balance wiring (confirmed + unconfirmed)')

// ── Rule 6: R1.6 address row wiring — confirmed hero + unconfirmed sub-line ──

const addressRowPath = path.join(componentsDir, 'address-row.tsx')
const addressesListPath = path.join(componentsDir, 'addresses-with-balance-list.tsx')
const addressRow = fs.readFileSync(addressRowPath, 'utf8')
const addressesList = fs.readFileSync(addressesListPath, 'utf8')

const rule6Violations: string[] = []
if (!addressRow.includes('confirmedSats')) {
	rule6Violations.push('address-row.tsx: missing confirmedSats prop')
}
if (!addressRow.includes('formatUnconfirmedBalanceLine')) {
	rule6Violations.push('address-row.tsx: must use formatUnconfirmedBalanceLine')
}
if (addressRow.includes('balanceSats')) {
	rule6Violations.push('address-row.tsx: must not use legacy balanceSats prop')
}
if (!addressesList.includes('confirmedSats={row.confirmedSats}')) {
	rule6Violations.push('addresses-with-balance-list.tsx: must forward confirmedSats to AddressRow')
}
// R1.7 pass 2: "·" interpunct replaced by a count badge chip (intentional design change).
if (!addressesList.includes('Addresses with balance')) {
	rule6Violations.push('addresses-with-balance-list.tsx: must keep the accordion header label')
}

assert.equal(
	rule6Violations.length,
	0,
	`Rule 6 violations — R1.6 addresses UX wiring:\n  ${rule6Violations.join('\n  ')}`,
)
console.log('Rule 6 PASS: address row wiring (confirmed + unconfirmed per address)')

// ── Rule 7: Phase 5 transactions wiring — panel renders the unconfirmed tx list ──

const panelDataPath = path.join(domainRoot, 'hooks', 'use-wallet-panel-data.ts')
const panelData = fs.readFileSync(panelDataPath, 'utf8')
const panelContentForTxs = fs.readFileSync(walletPanelContentPath, 'utf8')

const rule7Violations: string[] = []
if (!panelData.includes('useUnconfirmedTxs')) {
	rule7Violations.push('use-wallet-panel-data.ts: must compose useUnconfirmedTxs')
}
if (!panelData.includes('refreshUnconfirmedTxs()')) {
	rule7Violations.push('use-wallet-panel-data.ts: syncAndRefresh must refresh the unconfirmed tx list')
}
if (!panelContentForTxs.includes('UnconfirmedTxsList')) {
	rule7Violations.push('wallet-panel-content.tsx: must render UnconfirmedTxsList')
}
if (!panelContentForTxs.includes('isWatchOnly={isWatchOnly}')) {
	rule7Violations.push('wallet-panel-content.tsx: must forward isWatchOnly so Bump is disabled for watch-only')
}
if (!panelContentForTxs.includes('onAfterBump={onRefreshSync}')) {
	rule7Violations.push('wallet-panel-content.tsx: a successful bump must trigger the panel sync+refresh')
}

assert.equal(
	rule7Violations.length,
	0,
	`Rule 7 violations — Phase 5 transactions wiring:\n  ${rule7Violations.join('\n  ')}`,
)
console.log('Rule 7 PASS: unconfirmed transactions + fee-bump wiring')

// ── Rule 8: Phase 6 Send wiring — panel sub-view, capability gating, PRD copy ──

const sendFormPath = path.join(domainRoot, 'components', 'send-form.tsx')
const sendFormSource = fs.readFileSync(sendFormPath, 'utf8')
const sendCopyPath = path.join(domainRoot, 'model', 'format-send-error.ts')
const sendCopySource = fs.readFileSync(sendCopyPath, 'utf8')
const panelContentForSend = fs.readFileSync(walletPanelContentPath, 'utf8')

const rule8Violations: string[] = []
if (!panelContentForSend.includes('SendForm')) {
	rule8Violations.push('wallet-panel-content.tsx: must render SendForm for the send section')
}
if (!panelContentForSend.includes('isWatchOnly={isWatchOnly}')) {
	rule8Violations.push('wallet-panel-content.tsx: SendForm must receive the capability-derived isWatchOnly')
}
if (!panelContentForSend.includes('onAfterSend={onRefreshSync}')) {
	rule8Violations.push('wallet-panel-content.tsx: a successful send must trigger the panel sync+refresh')
}
if (!sendFormSource.includes('canConfirmSend')) {
	rule8Violations.push('send-form.tsx: Confirm must be gated by the canConfirmSend predicate (§4.3.5.5)')
}
// PRD §4.3.5.1 / §4.3.5.2 copy lives in exactly one audited file — literal drift fails CI.
if (!sendCopySource.includes("'Destination must be a bitcoin address.'")) {
	rule8Violations.push("format-send-error.ts: must contain the literal 'Destination must be a bitcoin address.'")
}
if (!sendCopySource.includes('`Destination must be a ${expectedNetwork} bitcoin address.`')) {
	rule8Violations.push('format-send-error.ts: must contain the wrong-network PRD copy template')
}
if (!sendCopySource.includes("'Insufficient funds'")) {
	rule8Violations.push("format-send-error.ts: must contain the literal 'Insufficient funds' (§4.3.5.2)")
}

assert.equal(rule8Violations.length, 0, `Rule 8 violations — Phase 6 Send wiring:\n  ${rule8Violations.join('\n  ')}`)
console.log('Rule 8 PASS: Send form wiring + PRD copy literals')

// ── Rule 9: Phase 7 wiring — Admin ID row (§4.1) + receive QR (§4.3.4.1) ──────

const adminIdRowPath = path.join(componentsDir, 'admin-id-row.tsx')
const adminIdPresentationPath = path.join(modelDir, 'admin-id-presentation.ts')
const receiveRowPath = path.join(componentsDir, 'receive-address-row.tsx')
const sessionControlPath = path.join(componentsDir, 'wallet-session-control.tsx')

const sharedAdminIdPath = path.join(domainRoot, '..', '..', 'lib', 'admin-id.ts')
const connectAdminIdCardPath = path.join(domainRoot, '..', 'connect-wallet', 'components', 'connect-admin-id-card.tsx')
const authoritySelectionPath = path.join(
	domainRoot,
	'..',
	'connect-wallet',
	'components',
	'authority-selection-phase.tsx',
)
const hwWalletConnectPath = path.join(domainRoot, '..', 'connect-wallet', 'components', 'hw-wallet-connect.tsx')
const authenticatePhasePath = path.join(
	domainRoot,
	'..',
	'connect-wallet',
	'components',
	'authenticate-session-phase.tsx',
)

const adminIdRow = fs.readFileSync(adminIdRowPath, 'utf8')
const adminIdPresentation = fs.readFileSync(adminIdPresentationPath, 'utf8')
const sharedAdminId = fs.readFileSync(sharedAdminIdPath, 'utf8')
const connectAdminIdCard = fs.readFileSync(connectAdminIdCardPath, 'utf8')
const authoritySelection = fs.readFileSync(authoritySelectionPath, 'utf8')
const hwWalletConnect = fs.readFileSync(hwWalletConnectPath, 'utf8')
const authenticatePhase = fs.readFileSync(authenticatePhasePath, 'utf8')
const receiveRow = fs.readFileSync(receiveRowPath, 'utf8')
const sessionControl = fs.readFileSync(sessionControlPath, 'utf8')
const panelContentForAdminId = fs.readFileSync(walletPanelContentPath, 'utf8')

const rule9Violations: string[] = []
// §4.1: the panel renders the Admin ID row and threads the auth address in.
if (!panelContentForAdminId.includes('AdminIdRow')) {
	rule9Violations.push('wallet-panel-content.tsx: must render AdminIdRow (§4.1)')
}
if (!panelContentForAdminId.includes('adminId={adminId}')) {
	rule9Violations.push('wallet-panel-content.tsx: must forward adminId to AdminIdRow')
}
if (!sessionControl.includes('adminId={adminId}')) {
	rule9Violations.push('wallet-session-control.tsx: must pass the session Admin ID into the panel content')
}
// PRD 06 §3.b.ii.2: every screen feeds the panel the Admin ID address, and none of them
// still threads a separate address alongside it — the Admin ID *is* that address, so a
// second field could only ever disagree with the first.
// `SessionChip` is listed alongside the panel components on purpose: manual-proposal-screen
// mounts the chip itself rather than through WalletSessionControl, and an earlier version of
// this rule keyed only on the panel — so that screen kept labelling its chip from the public
// key while the panel it opens showed the address. Same identity, two shapes, one click apart.
const screensDir = path.join(domainRoot, '..', '..', 'screens')
for (const entry of fs.readdirSync(screensDir)) {
	if (!entry.endsWith('.tsx')) continue
	const screen = fs.readFileSync(path.join(screensDir, entry), 'utf8')
	const mountsAdminId =
		screen.includes('WalletSessionControl') || screen.includes('WalletPanelContent') || screen.includes('SessionChip')
	if (!mountsAdminId) continue
	if (!screen.includes('adminId={wallet.addressSample}')) {
		rule9Violations.push(`screens/${entry}: must pass the Admin ID address into the panel (PRD 06 §3.b.ii.2)`)
	}
	if (screen.includes('adminIdAddress')) {
		rule9Violations.push(`screens/${entry}: must not thread a separate Admin ID address — the Admin ID is the address`)
	}
	// Formatting an Admin ID for display is one rule in `lib/admin-id.ts`, not a ternary per
	// screen. A screen that rolls its own is how the two shapes diverged in the first place.
	if (screen.includes('truncateAdminId(')) {
		rule9Violations.push(`screens/${entry}: must label the Admin ID via adminIdChipLabel, not its own truncation`)
	}
}
// Safety caption is a single audited literal, owned by the shared module the connect
// flow and the wallet panel both read (mirrors the §4.3.5 send-copy pattern).
if (!adminIdPresentation.includes("from '@/lib/admin-id'")) {
	rule9Violations.push('admin-id-presentation.ts: must re-export the shared Admin ID literals from @/lib/admin-id')
}
if (!sharedAdminId.includes('never send funds to this address.')) {
	rule9Violations.push('lib/admin-id.ts: must own the Admin ID safety caption literal (PRD 06 §3.b.ii.2)')
}
// The caption the app carried between PR #444 and PRD 06. Its absence is the pin: the
// Admin ID is an address again, and a surface that still calls it a public key would be
// describing the wrong thing to the signer.
if (sharedAdminId.includes('it is a public key, not a payment address.')) {
	rule9Violations.push('lib/admin-id.ts: the Admin ID is an address, not a public key (PRD 06 §3.b.ii.2)')
}
// §4.3.4.1: the receive row renders a real QR code.
if (!receiveRow.includes('QrCode')) {
	rule9Violations.push('receive-address-row.tsx: must render a QrCode (§4.3.4.1)')
}
// Signer safety: the Admin ID is auth-only and must NOT present a scannable QR.
if (adminIdRow.includes('QrCode')) {
	rule9Violations.push('admin-id-row.tsx: must NOT render a QR for the Admin ID (auth-only, never fundable)')
}
if (connectAdminIdCard.includes('QrCode')) {
	rule9Violations.push('connect-admin-id-card.tsx: must NOT render a QR for the Admin ID (auth-only, never fundable)')
}
// #410: the signer sees the Admin ID on the multisig-selection step, i.e. before the
// canonical signer-set membership check has resolved.
if (!authoritySelection.includes('<ConnectAdminIdCard adminId={adminId}')) {
	rule9Violations.push('authority-selection-phase.tsx: must render the Admin ID before the membership check (#410)')
}
// PRD 06 §3.b.ii.2: both connect steps show the address the device derived, and both
// read it from the same connect entry — the value the signer compares against the
// device screen must not depend on which step they are looking at.
const connectAdminIdSource = /adminId=\{state\.selectedEntry\.address\}/g
if ((hwWalletConnect.match(connectAdminIdSource) ?? []).length < 2) {
	rule9Violations.push(
		'hw-wallet-connect.tsx: both connect steps must be fed the Admin ID address from the connect entry (PRD 06 §3.b.ii.2)',
	)
}
if (authenticatePhase.includes('compressedPublicKey')) {
	rule9Violations.push('authenticate-session-phase.tsx: the Admin ID is an address, not a compressed public key')
}
// #409/#412: the device renders the Admin ID itself now, and PRD 06 §3.c.i puts that
// comparison inside the certificate modal as Step 2 — so the row offers one verify
// affordance, not two. The old "Address on device" block existed only to give the signer
// something to compare a raw public key against; keeping it would put the same string on
// screen twice.
if (adminIdRow.includes('VerifyOnDeviceButton')) {
	rule9Violations.push('admin-id-row.tsx: verify-on-device belongs to the certificate modal Step 2 (PRD 06 §3.c.i)')
}
if (adminIdRow.includes('Address on device')) {
	rule9Violations.push('admin-id-row.tsx: must not repeat the Admin ID under a second heading (#413)')
}
// Both Admin ID surfaces read the same audited literals.
if (!connectAdminIdCard.includes("from '@/lib/admin-id'")) {
	rule9Violations.push('connect-admin-id-card.tsx: must read the Admin ID literals from @/lib/admin-id')
}

assert.equal(
	rule9Violations.length,
	0,
	`Rule 9 violations — Phase 7 Admin ID + receive QR:\n  ${rule9Violations.join('\n  ')}`,
)
console.log('Rule 9 PASS: Admin ID row + receive QR wiring')

// ── Rule 10: G8 wiring — Admin ID Verification Certificate (PRD 06 §3.c.i) ────

const certificateModalPath = path.join(componentsDir, 'admin-id-certificate-modal.tsx')
const certificateModelPath = path.join(modelDir, 'admin-id-certificate.ts')
const certificateHookPath = path.join(domainRoot, 'hooks', 'use-admin-id-certificate.ts')

const certificateModal = fs.readFileSync(certificateModalPath, 'utf8')
const certificateModel = fs.readFileSync(certificateModelPath, 'utf8')
const certificateHook = fs.readFileSync(certificateHookPath, 'utf8')

const rule10Violations: string[] = []

// The three wireframes in docs/0-prd/assets/ are the UI contract for this modal, so their
// literals are pinned in the model and the modal must read them from there. A component
// that inlines its own copy is how a client-approved screen quietly drifts.
const wireframeLiterals = [
	'Generate Admin ID Verification Certificate',
	'Step 1. Sign Admin ID',
	'Waiting for signature to generate Admin ID Verification Certificate...',
	'Copied to clipboard',
]
for (const literal of wireframeLiterals) {
	if (!certificateModel.includes(literal)) {
		rule10Violations.push(`model/admin-id-certificate.ts: must own the wireframe literal "${literal}"`)
	}
	if (certificateModal.includes(`'${literal}'`) || certificateModal.includes(`>${literal}<`)) {
		rule10Violations.push(`admin-id-certificate-modal.tsx: must read "${literal}" from the model, not inline it`)
	}
}
if (!certificateModal.includes("from '../model/admin-id-certificate'")) {
	rule10Violations.push('admin-id-certificate-modal.tsx: must read its copy from model/admin-id-certificate')
}
// §3.c.i is satisfied by what the reader can do with the copied block, so the block's
// shape is a contract: message on line 1, signature on line 2, built in one place.
if (!certificateModal.includes('certificateBlock(')) {
	rule10Violations.push('admin-id-certificate-modal.tsx: must copy the certificate block, not the bare signature')
}
// The modal reuses the shared dialog and clipboard primitives — Escape/overlay close and
// the Linux clipboard-owner fix from #428 come with them.
if (!certificateModal.includes('AccessibleDialog')) {
	rule10Violations.push('admin-id-certificate-modal.tsx: must use AccessibleDialog (Escape / overlay close)')
}
if (!certificateModal.includes('useClipboardCopy')) {
	rule10Violations.push('admin-id-certificate-modal.tsx: must copy through useClipboardCopy (#428)')
}
// D5: all certificate cryptography lives in Rust. The frontend moves strings — it never
// builds the signed message (the `Admin ID: ` prefix is part of the signed bytes) and it
// never encodes a signature. This file is skipped: it names the forbidden shapes to find
// them.
const frontendSrcRoot = path.join(domainRoot, '..', '..')
const forbiddenInFrontend: Array<[string, string]> = [
	['Admin ID: $', 'the signed message is rendered in Rust'],
	["'Admin ID: '", 'the signed message is rendered in Rust'],
	['btoa(', 'the certificate is base64-encoded in Rust'],
]
const walk = (dir: string): string[] =>
	fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const full = path.join(dir, entry.name)
		if (entry.isDirectory()) return walk(full)
		return entry.name.endsWith('.ts') || entry.name.endsWith('.tsx') ? [full] : []
	})
for (const file of walk(frontendSrcRoot)) {
	if (file === new URL(import.meta.url).pathname || file.includes('__tests__')) continue
	const contents = fs.readFileSync(file, 'utf8')
	for (const [marker, why] of forbiddenInFrontend) {
		if (contents.includes(marker)) {
			rule10Violations.push(`${path.relative(frontendSrcRoot, file)}: ${why} (found "${marker}")`)
		}
	}
}
// The signature comes from the wallet-adapter port that already signs the session
// challenge; a second signing path would mean a second place to get device dispatch wrong.
if (!certificateHook.includes('adapter.signSighash(')) {
	rule10Violations.push('use-admin-id-certificate.ts: must sign through the wallet adapter port')
}
if (!certificateHook.includes('buildAdminIdCertificate(')) {
	rule10Violations.push('use-admin-id-certificate.ts: the certificate must be built (and verified) in Rust')
}
// Step 2 lives inside the modal and verifies the Admin ID as a P2WPKH address on the
// device. In a mnemonic session there is no screen to compare against, so the step is
// disabled with the reason rather than hidden (D3).
if (!certificateModal.includes('<VerifyOnDeviceButton')) {
	rule10Violations.push('admin-id-certificate-modal.tsx: Step 2 must verify the Admin ID on the device')
}
if (!certificateModal.includes('scriptType="p2wpkh"')) {
	rule10Violations.push('admin-id-certificate-modal.tsx: the Admin ID is verified as P2WPKH (PRD 06 §3.b.ii.2)')
}
if (!certificateModal.includes('CERTIFICATE_STEP_2_NO_DEVICE')) {
	rule10Violations.push('admin-id-certificate-modal.tsx: a mnemonic session must be told why Step 2 is disabled (D3)')
}
// Both Admin ID surfaces open the same modal — pre-sign-in (#410) and post-login (§4.a).
if (!adminIdRow.includes('<AdminIdCertificateModal')) {
	rule10Violations.push('admin-id-row.tsx: must offer the certificate modal (PRD 06 §4.a)')
}
// §3.c.i puts the certificate before sign-in as well, and #410 is explicit that the
// signer sees their Admin ID before the app judges its membership — so the affordance
// must sit on the connect card, which renders while that check is still running.
if (!connectAdminIdCard.includes('<AdminIdCertificateModal')) {
	rule10Violations.push('connect-admin-id-card.tsx: must offer the certificate before sign-in (PRD 06 §3.c.i, #410)')
}

assert.equal(
	rule10Violations.length,
	0,
	`Rule 10 violations — G8 Admin ID Verification Certificate:\n  ${rule10Violations.join('\n  ')}`,
)
console.log('Rule 10 PASS: Admin ID Verification Certificate wiring')

console.log('All architecture compliance checks passed.')
