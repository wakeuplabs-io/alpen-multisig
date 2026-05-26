import type { ApiResult } from '@/types'
import { tauriCall } from '@/api/tauri-bridge'

export type AdminWalletInfo = {
  address: string
  balance_sats: number
}

export function getAdminWalletInfo(): Promise<ApiResult<AdminWalletInfo>> {
  return tauriCall<AdminWalletInfo>('get_admin_wallet_info', {})
}
