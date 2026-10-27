import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'
import { adminIdsSchema } from '@/api/ipc-schemas'

/**
 * The Admin ID (P2WPKH address on the chain's network) of each signer key, derived in Rust. Keyed by the key as
 * `normalizePubkey` writes it; a key that is not a valid compressed public key is absent from the result.
 */
export function adminIdsForPubkeys(pubkeys: string[]): Promise<ApiResult<Record<string, string>>> {
	return tauriCall('admin_ids_for_pubkeys', { pubkeys }, adminIdsSchema)
}
