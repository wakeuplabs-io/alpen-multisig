import type { z } from 'zod'
import { feeRatesSchema } from '@/api/ipc-schemas'
import { tauriCall } from '@/api/tauri-bridge'
import type { ApiResult } from '@/types'

export type FeeRates = z.infer<typeof feeRatesSchema>

export function estimateFeeRates(): Promise<ApiResult<FeeRates>> {
	return tauriCall('fee_rates_estimate', {}, feeRatesSchema)
}
