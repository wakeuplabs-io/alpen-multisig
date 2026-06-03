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

console.log('All architecture compliance checks passed.')
