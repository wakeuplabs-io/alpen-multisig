import type { VkPredicateType } from '@/lib/vk-predicate'
import { VK_PREDICATE_TYPE_LABELS } from '@/lib/vk-predicate'
import type { ActionValidator } from './types'

const VK_CONDITION_HEX_LENGTH: Partial<Record<VkPredicateType, number>> = {
	bip340_schnorr: 64,
	sp1_groth16: 712,
}

export const validateVkUpdate: ActionValidator = ({ data, ctx }) => {
	const expectedLen = VK_CONDITION_HEX_LENGTH[data.vkTypeId]
	if (expectedLen === undefined) return

	const hex = data.newVkHex.trim()
	if (hex.length === 0) {
		ctx.addIssue({
			code: 'custom',
			path: ['newVkHex'],
			message: `Condition hex is required for ${VK_PREDICATE_TYPE_LABELS[data.vkTypeId]}`,
		})
	} else if (!/^[0-9a-fA-F]+$/.test(hex)) {
		ctx.addIssue({ code: 'custom', path: ['newVkHex'], message: 'Must be a valid hex string' })
	} else if (hex.length !== expectedLen) {
		ctx.addIssue({
			code: 'custom',
			path: ['newVkHex'],
			message: `${VK_PREDICATE_TYPE_LABELS[data.vkTypeId]} requires exactly ${expectedLen / 2} bytes (${expectedLen} hex chars), got ${hex.length / 2}`,
		})
	}
}
