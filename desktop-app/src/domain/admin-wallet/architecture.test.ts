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

console.log('All architecture compliance checks passed.')
