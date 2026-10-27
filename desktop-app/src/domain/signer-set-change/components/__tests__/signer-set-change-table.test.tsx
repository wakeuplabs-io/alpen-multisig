import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { renderToStaticMarkup } from 'react-dom/server'
import { SignerSetChangeTable } from '../signer-set-change-table.tsx'

const KEY_A = `02${'a'.repeat(64)}`
const KEY_B = `02${'b'.repeat(64)}`
const KEY_C = `02${'c'.repeat(64)}`

const html = renderToStaticMarkup(
	<SignerSetChangeTable
		change={{
			rows: [
				{ pubkey: KEY_A, inBefore: true, inAfter: true, isAdded: false, isRemoved: false },
				{ pubkey: KEY_B, inBefore: true, inAfter: false, isAdded: false, isRemoved: true },
				{ pubkey: KEY_C, inBefore: false, inAfter: true, isAdded: true, isRemoved: false },
			],
			thresholdBefore: 2,
			thresholdAfter: 2,
		}}
	/>,
)

assert.ok(html.includes('data-testid="e2e-signer-set-change-table"'), 'the shared E2E selector is stable')
assert.match(html, /<th[^>]*scope="col"[^>]*>Before<\/th>/, 'Before is a semantic column header')
assert.match(html, /<th[^>]*scope="col"[^>]*>After<\/th>/, 'After is a semantic column header')
// Signers read as their Admin ID (feedback item 60). A static render runs no effects, so every row shows the
// placeholder the Admin ID resolves into; the display rule itself is pinned in signer-admin-id.test.tsx.
for (const key of [KEY_A, KEY_B, KEY_C]) {
	assert.ok(!html.includes(key), 'a signer is never printed as its public key')
}
assert.ok(html.includes('−</span>…'), 'removed signers have a textual minus marker')
assert.ok(html.includes('line-through'), 'removed signers are struck through')
assert.ok(html.includes('+</span>…'), 'added signers have a textual plus marker')
assert.ok(html.includes('aria-label="Removed signer …"'), 'the removed-signer label names the Admin ID, not the key')
assert.ok(html.includes('>—</span>'), 'missing sides use an em dash rather than a blank cell')
assert.ok(html.includes('2 of 2'), 'threshold and signer cardinality are rendered together')

const __dirname = dirname(fileURLToPath(import.meta.url))
const domainRoot = join(__dirname, '..', '..', '..')
const consumers = [
	join(domainRoot, 'create-proposal', 'components', 'create-proposal-preview.tsx'),
	join(domainRoot, 'proposal-detail', 'components', 'proposal-detail.tsx'),
	join(domainRoot, 'cancel-proposal', 'components', 'cancel-target-summary.tsx'),
]

for (const consumer of consumers) {
	const source = readFileSync(consumer, 'utf8')
	assert.ok(source.includes('<SignerSetChangeTable'), `${consumer} must use the shared signer-set table`)
}

console.log('SignerSetChangeTable: rendering and shared wiring contracts passed')
