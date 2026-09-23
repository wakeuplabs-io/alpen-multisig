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
assert.ok(html.includes(KEY_A), 'unchanged keys remain reviewable')
assert.ok(html.includes(`−</span>${KEY_B}`), 'removed keys have a textual minus marker')
assert.ok(html.includes('line-through'), 'removed keys are struck through')
assert.ok(html.includes(`+</span>${KEY_C}`), 'added keys have a textual plus marker')
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
