// tauri drag-drop — file drops must reach the DOM (#606).
//
// Tauri 2 enables native drag-drop on every window by default. With it on, the webview
// swallows OS file drops to emit `tauri://drag-drop` to Rust, and the HTML5 `dragover` /
// `drop` events never fire. The manual proposal generator's "Drop bundle JSON here" zone
// listens for those DOM events, so it looked droppable and did nothing. Nothing in the app
// uses the native event, so every window turns it off.

import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const configPath = path.join(
	path.dirname(fileURLToPath(import.meta.url)),
	'..',
	'..',
	'..',
	'src-tauri',
	'tauri.conf.json',
)
const config = JSON.parse(fs.readFileSync(configPath, 'utf8')) as {
	app: { windows: { title?: string; dragDropEnabled?: boolean }[] }
}

assert.ok(config.app.windows.length > 0, 'tauri.conf.json must declare at least one window')
for (const window of config.app.windows) {
	assert.equal(
		window.dragDropEnabled,
		false,
		`window "${window.title}" must set dragDropEnabled: false so HTML5 file drops reach the DOM`,
	)
}
