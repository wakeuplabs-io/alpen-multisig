import { spawnSync } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = fileURLToPath(new URL('.', import.meta.url))

const MINE_URL = process.env.REGTEST_DEV_API_URL ?? 'http://127.0.0.1:3001'

/** Mine regtest blocks via regtest-dev-api HTTP endpoint. */
export function mineRegtestBlocks(count = 3) {
	const result = spawnSync('curl', ['-sf', '-X', 'POST', `${MINE_URL}/mine?count=${count}`], {
		encoding: 'utf8',
		env: process.env,
	})
	if (result.status !== 0) {
		throw new Error(result.stderr || result.stdout || `mine call failed (${result.status})`)
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
