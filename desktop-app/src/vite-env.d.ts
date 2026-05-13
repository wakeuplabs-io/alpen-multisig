/// <reference types="vite/client" />

interface ImportMetaEnv {
	readonly VITE_ORCHESTRATOR_BASE_URL?: string
	readonly VITE_BTC_RPC_URL?: string
	readonly VITE_BTC_RPC_USER?: string
	readonly VITE_BTC_RPC_PASS?: string
	readonly VITE_BTC_WALLET_NAME?: string
	readonly VITE_OPERATOR_SECRET_KEY_HEX?: string
	readonly VITE_MAGIC_BYTES_HEX?: string
	readonly VITE_ASM_RPC_URL?: string
	readonly VITE_BITCOIN_NETWORK?: string
}

interface ImportMeta {
	readonly env: ImportMetaEnv
}
