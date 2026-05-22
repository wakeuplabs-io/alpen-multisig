import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawn, spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const __dirname = fileURLToPath(new URL('.', import.meta.url))

/** Workspace root: desktop-app/e2e-webdriver -> .. -> .. */
const WORKSPACE_ROOT = path.resolve(__dirname, '../..')
const DESKTOP_BINARY = path.join(WORKSPACE_ROOT, 'target', 'debug', 'desktop-app')

let tauriDriver
let exit = false

function tauriDriverPath() {
	if (process.env.TAURI_DRIVER_PATH) {
		return process.env.TAURI_DRIVER_PATH
	}
	const home = path.join(os.homedir(), '.cargo', 'bin', 'tauri-driver')
	if (fs.existsSync(home)) {
		return home
	}
	return 'tauri-driver'
}

function closeTauriDriver() {
	exit = true
	tauriDriver?.kill('SIGTERM')
}

function registerSignalCleanup() {
	const stop = () => {
		closeTauriDriver()
	}
	process.once('SIGINT', stop)
	process.once('SIGTERM', stop)
}

registerSignalCleanup()

export const config = {
	runner: 'local',
	host: '127.0.0.1',
	port: 4444,
	specs: ['./test/specs/**/*.e2e.js'],
	maxInstances: 1,
	capabilities: [
		{
			maxInstances: 1,
			'tauri:options': {
				application: DESKTOP_BINARY,
			},
		},
	],
	reporters: ['spec'],
	framework: 'mocha',
	mochaOpts: {
		ui: 'bdd',
		// Broadcast spec waits on orchestrator commit/reveal confirmations (up to CONFIRM_TIMEOUT_MS).
		timeout: 600000,
	},

	onPrepare: () => {
		if (process.env.SKIP_E2E_BUILD === '1') {
			if (!fs.existsSync(DESKTOP_BINARY)) {
				throw new Error(`SKIP_E2E_BUILD=1 but binary missing: ${DESKTOP_BINARY}`)
			}
			return
		}
		// `cargo build -p desktop-app` alone may not re-embed the Vite bundle; use the Tauri CLI so
		// `beforeBuildCommand` runs and the WebView loads the same assets as production-ish builds.
		const desktopAppDir = path.join(WORKSPACE_ROOT, 'desktop-app')
		const r = spawnSync('npm', ['run', 'tauri', 'build', '--', '--debug', '--no-bundle'], {
			cwd: desktopAppDir,
			stdio: 'inherit',
			shell: true,
		})
		if (r.status !== 0) {
			throw new Error(`tauri build --debug --no-bundle failed with exit ${r.status}`)
		}
		if (!fs.existsSync(DESKTOP_BINARY)) {
			throw new Error(`Expected binary not found after build: ${DESKTOP_BINARY}`)
		}
	},

	beforeSession: () => {
		tauriDriver = spawn(tauriDriverPath(), [], {
			stdio: [null, process.stdout, process.stderr],
		})
		tauriDriver.on('error', (error) => {
			console.error('tauri-driver error:', error)
			process.exit(1)
		})
		tauriDriver.on('exit', (code) => {
			if (!exit) {
				console.error('tauri-driver exited with code:', code)
				process.exit(1)
			}
		})
	},

	afterSession: () => {
		closeTauriDriver()
	},
}
