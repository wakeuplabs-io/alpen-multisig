import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'

export type MultisigConfig = {
  signers: string[]
  threshold: number
}

export function getMultisigConfig(authority: string): Promise<ApiResult<MultisigConfig>> {
  return tauriCall<MultisigConfig>('get_multisig_config', { authority })
}
