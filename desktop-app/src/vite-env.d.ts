/// <reference types="vite/client" />

interface ImportMetaEnv {
	readonly VITE_ORCHESTRATOR_BASE_URL?: string
}

interface ImportMeta {
	readonly env: ImportMetaEnv
}

declare const __APP_VERSION__: string
