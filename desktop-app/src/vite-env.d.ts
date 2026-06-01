/// <reference types="vite/client" />
/// <reference types="vitest/globals" />
/// <reference types="@testing-library/jest-dom/vitest" />

interface ImportMetaEnv {
	readonly VITE_ORCHESTRATOR_BASE_URL?: string
}

interface ImportMeta {
	readonly env: ImportMetaEnv
}
