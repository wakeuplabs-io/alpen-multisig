// Every SafeHarbourNote render site must be gated on the live chain read.
//
// `SafeHarbourNote` is purely presentational — it says "Safe harbour is already active" whenever
// it is mounted. The send screen mounted it for any Defcon 1 proposal without reading the chain,
// so a freshly started stack (harbour down) told the council their emergency lever was already
// pulled and would only burn a sequence number and fees. Nothing in the types can express "this
// note needs a read behind it", so the contract is guarded here.
//
// Source-text assertions, following the project's existing tsx-runner style — React rendering
// tests need vitest + @testing-library/react (BLOCKED_BY_DEPENDENCY — not installed).

import assert from 'node:assert/strict'
import { readdirSync, readFileSync, statSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const srcDir = join(dirname(fileURLToPath(import.meta.url)), '..', '..')
const componentPath = join(srcDir, 'components', 'safe-harbour-note.tsx')

function walk(dir: string): string[] {
	return readdirSync(dir).flatMap((entry) => {
		const full = join(dir, entry)
		return statSync(full).isDirectory() ? walk(full) : [full]
	})
}

const renderSites = walk(srcDir)
	.filter((file) => file.endsWith('.tsx') && file !== componentPath)
	.map((file) => ({ file, source: readFileSync(file, 'utf8') }))
	.filter(({ source }) => source.includes('<SafeHarbourNote'))

assert.ok(renderSites.length > 0, 'expected at least one SafeHarbourNote render site to guard')

for (const { file, source } of renderSites) {
	const name = relative(srcDir, file)

	assert.ok(
		source.includes('useSafeHarbourActivated('),
		`${name}: renders SafeHarbourNote, so it must read the harbour state from the node`,
	)

	// The note asserts a fact about the chain; an ungated mount asserts it unconditionally.
	for (const match of source.matchAll(/<SafeHarbourNote/g)) {
		const preceding = source.slice(Math.max(0, (match.index ?? 0) - 400), match.index)
		assert.ok(
			preceding.includes('safeHarbourActivated'),
			`${name}: SafeHarbourNote must be rendered behind the safeHarbourActivated guard`,
		)
	}
}

console.log(`safe-harbour-note gating: ${renderSites.length} render sites guarded`)
