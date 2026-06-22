import type { ActionValidator } from './types'
import { EVEN_PUBKEY_HEX_PATTERN, U32_MAX } from './types'

export const validateOperatorSetUpdate: ActionValidator = ({ data, ctx }) => {
	const filledAdd = data.operatorsToAdd.filter((r) => r.value.trim().length > 0)
	const filledRemove = data.operatorIndicesToRemove.filter((r) => r.value.trim().length > 0)
	if (filledAdd.length === 0 && filledRemove.length === 0) {
		ctx.addIssue({
			code: 'custom',
			path: ['operatorsToAdd'],
			message: 'At least one operator must be added or removed',
		})
	}

	const addKeyIndexes = new Map<string, number[]>()
	for (const [index, row] of data.operatorsToAdd.entries()) {
		const key = row.value.trim()
		if (key.length === 0) continue
		if (!EVEN_PUBKEY_HEX_PATTERN.test(key)) {
			ctx.addIssue({
				code: 'custom',
				path: ['operatorsToAdd', index, 'value'],
				message: 'Operator key must be an x-only pubkey hex (32 bytes, 64 hex chars, no 02/03 prefix)',
			})
		}
		const normalized = key.toLowerCase()
		const indexes = addKeyIndexes.get(normalized) ?? []
		indexes.push(index)
		addKeyIndexes.set(normalized, indexes)
	}
	for (const indexes of addKeyIndexes.values()) {
		if (indexes.length < 2) continue
		for (const index of indexes) {
			ctx.addIssue({ code: 'custom', path: ['operatorsToAdd', index, 'value'], message: 'Duplicate operator key' })
		}
	}

	const removeIndexTracker = new Map<number, number[]>()
	for (const [index, row] of data.operatorIndicesToRemove.entries()) {
		const raw = row.value.trim()
		if (raw.length === 0) continue
		const n = Number(raw)
		if (!Number.isInteger(n) || n < 0 || n > U32_MAX) {
			ctx.addIssue({
				code: 'custom',
				path: ['operatorIndicesToRemove', index, 'value'],
				message: `Operator index must be a non-negative integer (0–${U32_MAX})`,
			})
			continue
		}
		const positions = removeIndexTracker.get(n) ?? []
		positions.push(index)
		removeIndexTracker.set(n, positions)
	}
	for (const positions of removeIndexTracker.values()) {
		if (positions.length < 2) continue
		for (const index of positions) {
			ctx.addIssue({
				code: 'custom',
				path: ['operatorIndicesToRemove', index, 'value'],
				message: 'Duplicate operator index',
			})
		}
	}
}
