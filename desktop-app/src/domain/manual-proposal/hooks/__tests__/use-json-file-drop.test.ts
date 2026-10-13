// use-json-file-drop — a bundle dropped on the manual proposal generator must load (#606).
//
// HTML5 `drop` events never carry files in this app: WebKitGTK (Linux) does not deliver
// dropped files to the DOM, and Windows blocks them while Tauri's native drag-drop is on.
// The first fix turned native drag-drop off and kept the HTML handler; on Linux the zone
// still highlighted but the drop did nothing. What is pinned here is the wiring that works
// everywhere: native drag-drop stays on, the window may listen for its events, and the form
// loads drops through the native event instead of `onDrop`.

import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const appRoot = path.join(here, '..', '..', '..', '..', '..')

/** Drops comments so prose mentioning an API is not mistaken for a call to it. */
function code(filePath: string): string {
	return fs
		.readFileSync(filePath, 'utf8')
		.replace(/\/\*[\s\S]*?\*\//g, '')
		.replace(/^\s*\/\/.*$/gm, '')
}

const config = JSON.parse(fs.readFileSync(path.join(appRoot, 'src-tauri', 'tauri.conf.json'), 'utf8')) as {
	app: { windows: { title?: string; label?: string; dragDropEnabled?: boolean }[] }
}
for (const window of config.app.windows) {
	assert.notEqual(
		window.dragDropEnabled,
		false,
		`window "${window.title}" must keep native drag-drop on — it is the only way dropped files arrive`,
	)
}

// onDragDropEvent is built on `listen`; without these permissions it rejects and drops are lost.
const capability = JSON.parse(
	fs.readFileSync(path.join(appRoot, 'src-tauri', 'capabilities', 'default.json'), 'utf8'),
) as { windows: string[]; permissions: string[] }
const mainLabel = config.app.windows[0]?.label ?? 'main'
assert.ok(capability.windows.includes(mainLabel), `the capability must apply to the "${mainLabel}" window`)
for (const permission of ['core:event:allow-listen', 'core:event:allow-unlisten']) {
	assert.ok(capability.permissions.includes(permission), `the capability must grant ${permission}`)
}

const hook = code(path.join(here, '..', 'use-json-file-drop.ts'))
assert.match(hook, /onDragDropEvent\(/, 'the hook must load drops from the native drag-drop event')
assert.match(hook, /readDroppedJson\(/, 'the hook must read the dropped file through the IPC')
assert.match(hook, /new File\(/, 'the hook must hand over a File, the same shape the picker produces')

const form = code(path.join(here, '..', '..', 'components', 'manual-import-form.tsx'))
assert.match(form, /useJsonFileDrop\(/, 'the import form must load drops through useJsonFileDrop')
assert.doesNotMatch(form, /onDrop=/, 'the import form must not rely on the HTML5 drop event')
