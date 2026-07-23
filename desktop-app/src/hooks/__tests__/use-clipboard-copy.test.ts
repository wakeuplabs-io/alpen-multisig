// use-clipboard-copy — a failed copy must be reported, never swallowed (#428).
//
// The hook's React wiring is not exercised here (no DOM in this runner); what is
// pinned is the contract the UI depends on: the module reaches the clipboard through
// the Tauri IPC and never through `navigator.clipboard`, and the returned shape
// carries an `error` channel so a failure can be shown.

import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

/** Drops comments so prose mentioning an API is not mistaken for a call to it. */
function code(filePath: string): string {
	return fs
		.readFileSync(filePath, 'utf8')
		.replace(/\/\*[\s\S]*?\*\//g, '')
		.replace(/^\s*\/\/.*$/gm, '')
}

const hookPath = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'use-clipboard-copy.ts')
const source = code(hookPath)

// Copies go through the IPC bridge. `navigator.clipboard` can be rejected by the
// WebView, which is why every copy surface was consolidated onto this path.
assert.match(source, /writeClipboard/, 'the hook must write through the Tauri IPC bridge')
assert.doesNotMatch(source, /navigator\.clipboard/, 'the hook must not use navigator.clipboard')

// A rejection must be handled — either a second `then` argument or a `.catch`.
// Without it a failed copy leaves the button looking inert, which is exactly how
// #428 was reported.
assert.match(source, /\(reason: unknown\) =>|\.catch\(/, 'the hook must handle a rejected clipboard write')

// The error must be observable by callers.
assert.match(source, /error: string \| null/, 'ClipboardCopy must expose an error channel')
assert.match(source, /setError\(/, 'the hook must record the failure')
assert.match(source, /return \{ copied, error, copy \}/, 'the hook must return the error to callers')

// Empty text stays a no-op — nothing to copy, and no false "Copied!" feedback.
assert.match(source, /if \(!text\) return/, 'empty text must not report a successful copy')

// No copy surface anywhere in the app may bypass the bridge.
const srcRoot = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '..')
const offenders: string[] = []

function walk(dir: string) {
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const full = path.join(dir, entry.name)
		if (entry.isDirectory()) {
			walk(full)
		} else if (/\.tsx?$/.test(full) && !full.includes('__tests__') && !full.endsWith('.test.ts')) {
			if (code(full).includes('navigator.clipboard')) {
				offenders.push(path.relative(srcRoot, full))
			}
		}
	}
}
walk(srcRoot)

assert.deepEqual(offenders, [], `these files bypass the clipboard bridge:\n  ${offenders.join('\n  ')}`)

console.log('use-clipboard-copy: all assertions passed')
