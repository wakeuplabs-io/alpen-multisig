#!/usr/bin/env node
/**
 * Runs every unit test under `src/` — one file, one process.
 *
 * The CI workflow used to name each test file by hand, and `package.json` carried one `test:*`
 * script per file to name it with. That list drifted: 21 of the 62 scripts were never invoked by
 * CI, and a slice that added three new tests needed a follow-up commit to remember to run them.
 * A test that does not run is not evidence of anything, so the list is now the filesystem.
 *
 * Each file is a standalone script of top-level `node:assert` calls, so a separate process per
 * file is what keeps one file's module state out of the next one's.
 */
import { spawnSync } from 'node:child_process'
import { readdirSync } from 'node:fs'
import { join } from 'node:path'

const ROOT = 'src'
const IS_TEST = /\.test\.tsx?$/

function testFiles(dir) {
	return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
		const path = join(dir, entry.name)
		if (entry.isDirectory()) return testFiles(path)
		return IS_TEST.test(entry.name) ? [path] : []
	})
}

const files = testFiles(ROOT).sort()
if (files.length === 0) {
	console.error(`No test files found under ${ROOT}/ — the glob is wrong, not the suite.`)
	process.exit(1)
}

const failed = []
for (const file of files) {
	const result = spawnSync('npx', ['tsx', file], { stdio: 'inherit' })
	if (result.status !== 0) failed.push(file)
}

console.log(`\n${files.length - failed.length}/${files.length} test files passed`)
if (failed.length > 0) {
	console.error(`\nFailed:\n${failed.map((f) => `  ${f}`).join('\n')}`)
	process.exit(1)
}
