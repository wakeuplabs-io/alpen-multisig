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
