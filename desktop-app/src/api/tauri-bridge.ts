import { invoke } from '@tauri-apps/api/core'
import type { ApiResult } from '@/types'

// Thin wrapper around Tauri's invoke() that normalises results into ApiResult<T>.
// Use this instead of fetch() — the Rust side handles auth headers, base URL, and token storage.
//
// Example:
//   const result = await tauriCall<Proposal[]>('list_proposals', { status: 'pending' })
//   if (result.ok) console.log(result.data)

export async function tauriCall<T>(
	command: string,
	args?: Record<string, unknown>
): Promise<ApiResult<T>> {
	try {
		const data = await invoke<T>(command, args)
		return { ok: true, data }
	} catch (err) {
		return { ok: false, error: err instanceof Error ? err.message : String(err) }
	}
}
