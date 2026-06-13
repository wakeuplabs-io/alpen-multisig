import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'
import { createRequire } from 'module'

const { version } = createRequire(import.meta.url)('./package.json')

// https://vitejs.dev/config/
export default defineConfig({
	plugins: [react()],
	define: {
		__APP_VERSION__: JSON.stringify(version),
	},
	resolve: {
		alias: {
			'@': path.resolve(__dirname, './src'),
		},
	},
	// Tauri renders in a modern evergreen webview (WebView2 / WebKitGTK), so the
	// browser-matrix default target is unnecessary — and esbuild >= 0.28 (forced
	// via the root `overrides` for GHSA-gv7w-rqvm-qjhr) no longer down-levels
	// syntax to those old targets. Apply es2022 to both the build output and
	// the dev-server pre-bundling (optimizeDeps) paths so packages like zod v4
	// and react-hook-form v7.75+ don't hit the same esbuild transform error.
	build: {
		target: 'es2022',
	},
	optimizeDeps: {
		esbuildOptions: {
			target: 'es2022',
		},
	},
	// Prevent vite from obscuring Rust errors
	clearScreen: false,
	// Tauri expects a fixed port
	server: {
		port: 1420,
		strictPort: true,
		watch: {
			// On Windows, this is required
			ignored: ['**/src-tauri/**'],
		},
	},
})
