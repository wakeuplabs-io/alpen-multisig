import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = fileURLToPath(new URL('.', import.meta.url))

function runtestsDir() {
	if (process.env.ALPEN_RUNTESTS_DIR) {
		return process.env.ALPEN_RUNTESTS_DIR
	}
	return path.resolve(__dirname, '../../../../../runtests')
}

function autotestEnvDefaults() {
	if (process.env.ALPEN_AUTOTEST_DIR) {
		return path.join(process.env.ALPEN_AUTOTEST_DIR, 'env.defaults.sh')
	}
	return path.resolve(__dirname, '../../../../../autotest/env.defaults.sh')
}

/** Mine regtest blocks (orchestrator commit/reveal need ≥1 conf each). */
export function mineRegtestBlocks(count = 3) {
	const script = path.join(runtestsDir(), 'mine-blocks.sh')
	const envDefaults = autotestEnvDefaults()
	const runtests = runtestsDir()
	const result = spawnSync(
		'bash',
		[
			'-lc',
			`set -euo pipefail
[ -f "${envDefaults}" ] && source "${envDefaults}"
source "${runtests}/env.sh"
exec "${script}" ${String(count)}`,
		],
		{ encoding: 'utf8', env: process.env },
	)
	if (result.status !== 0) {
		throw new Error(result.stderr || result.stdout || `mine-blocks.sh failed (${result.status})`)
	}
	return result.stdout.trim()
}

/**
 * After Confirm & Broadcast, orchestrator polls until commit/reveal confirm.
 * On regtest nothing confirms without generatetoaddress — mine until the UI is done.
 */
export async function mineWhileWaitingForBroadcastDone(doneSelector, options = {}) {
	const timeoutMs = options.timeoutMs ?? 240000
	const pollMs = options.pollMs ?? 15000
	const blocksPerRound = options.blocksPerRound ?? 2
	const initialBlocks = options.initialBlocks ?? 3

	const done = await $(doneSelector)
	mineRegtestBlocks(initialBlocks)

	const deadline = Date.now() + timeoutMs
	while (Date.now() < deadline) {
		if (await done.isDisplayed()) {
			return
		}
		try {
			await done.waitForDisplayed({ timeout: pollMs })
			return
		} catch {
			mineRegtestBlocks(blocksPerRound)
		}
	}
	await done.waitForDisplayed({
		timeout: 5000,
		timeoutMsg: 'Broadcast done banner not shown — mine regtest blocks during commit/reveal wait',
	})
}
