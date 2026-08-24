// color-tokens — red means error, and nothing else (#416).
//
// The client asked twice for this. The first fix swapped hex in one screen while
// the same warm treatment lived on in ~57 files, so the complaint came back. What
// makes the rule hold is not the swap but this check: `styles.css` owns every
// red/amber value, and a component that wants one has to say *why* by picking a
// token — `danger` for failures, `emphasis` for everything that merely stands out.
//
// The check used to compare against a list of known hexes, so a red nobody had
// listed walked straight through it (#492 — `#c0392b`). It now recognises the
// family by hue and saturation instead, so any red/amber/orange is caught on
// sight, listed or not.
//
// If this test fails, do not look for a way to exempt the value. Pick the token
// that describes the intent; if none fits, the intent is probably not "error".

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
 * Hexes we have already replaced once, and the token each one became. Only a
 * hint for the failure message — detection is by color family below, so an
 * unlisted red is caught just the same.
 */
const KNOWN_REPLACEMENTS: Record<string, string> = {
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

/**
 * Six-digit hexes anywhere, plus three-digit ones only inside a Tailwind
 * arbitrary value (`text-[#f00]`). Bare `#416` in code is an issue reference,
 * not a color.
 */
const HEX_LITERAL = /#[0-9a-fA-F]{6}\b|(?<=-\[)#[0-9a-fA-F]{3}(?=\])/g

/** Saturation below this reads as grey, whatever its nominal hue. */
const GREY_BELOW = 0.12

/** `styles.css` is the one place a raw value may live — it defines the tokens. */
const OWNS_THE_PALETTE = 'styles.css'

/** Hue in degrees and saturation in 0..1, straight from the HSV conversion. */
function hueAndSaturation(hex: string): { hue: number; saturation: number } {
	const full =
		hex.length === 4
			? hex
					.slice(1)
					.split('')
					.map((c) => c + c)
					.join('')
			: hex.slice(1)
	const r = parseInt(full.slice(0, 2), 16) / 255
	const g = parseInt(full.slice(2, 4), 16) / 255
	const b = parseInt(full.slice(4, 6), 16) / 255
	const max = Math.max(r, g, b)
	const delta = max - Math.min(r, g, b)

	let hue = 0
	if (delta !== 0) {
		if (max === r) hue = 60 * (((g - b) / delta) % 6)
		else if (max === g) hue = 60 * ((b - r) / delta + 2)
		else hue = 60 * ((r - g) / delta + 4)
	}
	if (hue < 0) hue += 360

	return { hue, saturation: max === 0 ? 0 : delta / max }
}

/** The red-through-yellow arc, ignoring greys. Greens, blues and violets pass. */
function isWarm(hex: string): boolean {
	const { hue, saturation } = hueAndSaturation(hex)
	return saturation >= GREY_BELOW && (hue <= 60 || hue >= 345)
}

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

		for (const match of source.matchAll(HEX_LITERAL)) {
			const hex = match[0].toLowerCase()
			if (!isWarm(hex)) continue
			const line = source.slice(0, match.index).split('\n').length
			const token = KNOWN_REPLACEMENTS[hex] ?? 'a danger/emphasis/accent token'
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

console.log('color-tokens: no red/amber outside styles.css (detected by hue, not by allowlist)')
