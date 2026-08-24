/**
 * System clipboard access for the e2e specs.
 *
 * The failure #428 reported lives in the app → OS hand-off, so the specs read the real
 * system clipboard rather than anything the WebView reports about itself.
 */
import { execFileSync, spawn } from 'node:child_process'

/** Reads the system clipboard, preferring Wayland and falling back to X11. */
export function readSystemClipboard() {
	const readers = [
		['wl-paste', ['--no-newline']],
		['xclip', ['-selection', 'clipboard', '-o']],
		['xsel', ['--clipboard', '--output']],
	]
	for (const [bin, args] of readers) {
		try {
			return execFileSync(bin, args, { encoding: 'utf8', timeout: 5000 }).trim()
		} catch {
			// Try the next reader — the session may be Wayland-only or X11-only.
		}
	}
	throw new Error('no clipboard reader available (install wl-clipboard, xclip or xsel)')
}

/**
 * Puts a known value on the clipboard so a no-op copy is distinguishable from a
 * stale hit. Clipboard writers stay alive to *serve* the selection, so they must
 * run detached rather than being waited on (a synchronous call just times out).
 */
export async function seedClipboard(value) {
	const writers = [
		['wl-copy', [value]],
		['xclip', ['-selection', 'clipboard', '-i']],
	]
	for (const [bin, args] of writers) {
		const child = spawn(bin, args, { stdio: ['pipe', 'ignore', 'ignore'], detached: true })
		let spawnFailed = false
		child.on('error', () => {
			spawnFailed = true
		})
		child.stdin.end(value)
		child.unref()

		// Give the writer a moment to take ownership, then confirm it stuck.
		await new Promise((resolve) => setTimeout(resolve, 500))
		if (!spawnFailed && readSystemClipboard() === value) {
			return
		}
	}
	throw new Error('no clipboard writer available (install wl-clipboard or xclip)')
}
