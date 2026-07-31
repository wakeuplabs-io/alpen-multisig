// color-tokens — red means error, and nothing else (#416).
//
// The client asked twice for this. The first fix swapped hex in one screen while
// the same warm treatment lived on in ~57 files, so the complaint came back. What
// makes the rule hold is not the swap but this check: `styles.css` owns every
// red/amber value, and a component that wants one has to say *why* by picking a
// token — `danger` for failures, `emphasis` for everything that merely stands out.
//
// If this test fails, do not add the hex to the allowlist. Pick the token that
// describes the intent; if none fits, the intent is probably not "error".

import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const srcRoot = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '..')

/** Drops comments so prose quoting a hex is not mistaken for a use of it. */
function code(filePath: string): string {
	return fs
		.readFileSync(filePath, 'utf8')
		.replace(/\/\*[\s\S]*?\*\//g, '')
		.replace(/^\s*\/\/.*$/gm, '')
		.replace(/\{\/\*[\s\S]*?\*\/\}/g, '')
}

/**
 * The red/amber/warm family, and the token that replaces each one. Values that
 * only ever meant "error" map to `danger-*`; the warm tones that were doing
 * emphasis map to `emphasis*` or the brand `accent-border`.
 */
const FORBIDDEN: Record<string, string> = {
	'#dc2626': 'text-danger / border-danger',
	'#b91c1c': 'text-danger-strong',
	'#991b1b': 'text-danger-deep',
	'#7f1d1d': 'text-danger-deep',
	'#ef4444': 'text-danger',
	'#f87171': 'text-danger',
	'#fca5a5': 'border-danger-border-soft',
	'#fecaca': 'border-danger-border',
	'#fee2e2': 'bg-danger-surface',
	'#fef2f2': 'bg-danger-surface',
	'#92400e': 'text-emphasis',
	'#b45309': 'text-emphasis-soft',
	'#c2773b': 'text-emphasis-soft',
	'#d97706': 'text-emphasis-soft',
	'#c2410c': 'text-emphasis-soft',
	'#ea580c': 'text-emphasis-soft',
	'#f59e0b': 'text-emphasis-soft',
	'#fbbf24': 'border-accent-border',
	'#fcd34d': 'border-accent-border',
	'#fde68a': 'border-accent-border',
	'#fef3c7': 'bg-highlight-surface-alt',
	'#fffbeb': 'bg-highlight-surface',
	'#fefce8': 'bg-highlight-surface',
	'#fffdf5': 'bg-highlight-surface',
}

/** Tailwind's own red/amber/orange utilities are the same problem by another spelling. */
const FORBIDDEN_UTILITIES = /\b(?:text|bg|border|ring|from|to|via)-(?:red|amber|orange|yellow)-\d{2,3}\b/

/** `styles.css` is the one place a raw value may live — it defines the tokens. */
const OWNS_THE_PALETTE = 'styles.css'

const offenders: string[] = []

function walk(dir: string) {
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const full = path.join(dir, entry.name)
		if (entry.isDirectory()) {
			walk(full)
			continue
		}
		if (!/\.tsx?$/.test(full) || full.includes('__tests__') || full.endsWith('.test.ts')) continue

		const source = code(full)
		const rel = path.relative(srcRoot, full)

		for (const [hex, token] of Object.entries(FORBIDDEN)) {
			const at = source.toLowerCase().indexOf(hex)
			if (at === -1) continue
			const line = source.slice(0, at).split('\n').length
			offenders.push(`${rel}:${line} uses ${hex} — use ${token}`)
		}

		const utility = FORBIDDEN_UTILITIES.exec(source)
		if (utility !== null) {
			const line = source.slice(0, utility.index).split('\n').length
			offenders.push(`${rel}:${line} uses ${utility[0]} — use a danger/emphasis token`)
		}
	}
}
walk(srcRoot)

assert.deepEqual(
	offenders,
	[],
	`red/amber values must live in ${OWNS_THE_PALETTE} and reach components through a token:\n  ${offenders.join('\n  ')}`,
)

// The tokens themselves must exist, or every call site above resolves to nothing
// and the sweep silently un-styles the app.
const theme = fs.readFileSync(path.join(srcRoot, 'styles.css'), 'utf8')
for (const token of [
	'--color-danger',
	'--color-danger-strong',
	'--color-danger-deep',
	'--color-danger-surface',
	'--color-danger-border',
	'--color-danger-border-soft',
	'--color-emphasis',
	'--color-emphasis-soft',
	'--color-highlight-surface',
	'--color-highlight-surface-alt',
]) {
	assert.match(theme, new RegExp(`${token}:`), `styles.css must define ${token}`)
}

console.log(`color-tokens: ${Object.keys(FORBIDDEN).length} red/amber values confined to styles.css`)
