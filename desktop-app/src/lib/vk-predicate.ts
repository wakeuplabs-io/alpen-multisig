export const VK_PREDICATE_TYPES = ['always_accept', 'never_accept', 'bip340_schnorr', 'sp1_groth16'] as const
export type VkPredicateType = (typeof VK_PREDICATE_TYPES)[number]

export const VK_PREDICATE_TYPE_IDS: Record<VkPredicateType, number> = {
	always_accept: 1,
	never_accept: 0,
	bip340_schnorr: 10,
	sp1_groth16: 20,
}

export const VK_PREDICATE_TYPE_LABELS: Record<VkPredicateType, string> = {
	always_accept: 'AlwaysAccept',
	never_accept: 'NeverAccept',
	bip340_schnorr: 'Bip340Schnorr',
	sp1_groth16: 'Sp1Groth16',
}

export function vkPredicateLabelFromTypeId(typeId: number): string {
	for (const t of VK_PREDICATE_TYPES) {
		if (VK_PREDICATE_TYPE_IDS[t] === typeId) return VK_PREDICATE_TYPE_LABELS[t]
	}
	return `Type ${typeId}`
}
