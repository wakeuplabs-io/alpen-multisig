import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { renderToStaticMarkup } from 'react-dom/server'
import { SignerSetChangeTable } from '../signer-set-change-table.tsx'

// Three regtest Admin IDs: the rows carry what the ASM lists.
const KEY_A = 'bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080'
const KEY_B = 'bcrt1qq6hag67dl53wl99vzg42z8eyzfz2xlkvwk6f7m'
const KEY_C = 'bcrt1q86edcefehpvevtaaqm8acjtxkakj04nska9c5y'

const html = renderToStaticMarkup(
	<SignerSetChangeTable
		change={{
			rows: [
				{ adminId: KEY_A, inBefore: true, inAfter: true, isAdded: false, isRemoved: false },
				{ adminId: KEY_B, inBefore: true, inAfter: false, isAdded: false, isRemoved: true },
				{ adminId: KEY_C, inBefore: false, inAfter: true, isAdded: true, isRemoved: false },
			],
			thresholdBefore: 2,
			thresholdAfter: 2,
		}}
	/>,
)

assert.ok(html.includes('data-testid="e2e-signer-set-change-table"'), 'the shared E2E selector is stable')
assert.match(html, /<th[^>]*scope="col"[^>]*>Before<\/th>/, 'Before is a semantic column header')
assert.match(html, /<th[^>]*scope="col"[^>]*>After<\/th>/, 'After is a semantic column header')
// Signers read as their Admin ID (feedback item 60), printed straight from the row: no lookup, no placeholder.
for (const id of [KEY_A, KEY_B, KEY_C]) {
	assert.ok(html.includes(id), 'every signer prints its Admin ID')
}
assert.ok(!html.includes('…'), 'no row waits on a resolution')
assert.ok(html.includes(`−</span>${KEY_B}`), 'removed signers have a textual minus marker')
assert.ok(html.includes('line-through'), 'removed signers are struck through')
assert.ok(html.includes(`+</span>${KEY_C}`), 'added signers have a textual plus marker')
assert.ok(html.includes(`aria-label="Removed signer ${KEY_B}"`), 'the removed-signer label names the Admin ID')
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
