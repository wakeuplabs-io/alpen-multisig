import { invoke } from '@tauri-apps/api/core'
import type { z } from 'zod'
import type { ApiResult } from '@/types'

export async function writeClipboard(text: string): Promise<void> {
  await invoke('write_clipboard', { text })
}

export async function saveJsonFile(content: string, filename: string): Promise<void> {
  await invoke('save_json_file', { content, filename })
}

// Thin wrapper around Tauri's invoke() that normalises results into ApiResult<T>.
// Pass a Zod schema to reject unknown IPC enum variants at the boundary (P-008).

export async function tauriCall<T>(
	command: string,
	args?: Record<string, unknown>,
	schema?: z.ZodType<T>,
): Promise<ApiResult<T>> {
	try {
		const raw = await invoke<unknown>(command, args)
		if (schema !== undefined) {
			const parsed = schema.safeParse(raw)
			if (!parsed.success) {
				return {
					ok: false,
					error: `Invalid IPC response for ${command}: ${parsed.error.message}`,
					errorCode: 'ipc_schema_mismatch',
				}
			}
			return { ok: true, data: parsed.data }
		}
		return { ok: true, data: raw as T }
	} catch (err) {
		return { ok: false, error: err instanceof Error ? err.message : String(err) }
	}
}
