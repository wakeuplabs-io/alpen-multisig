import type { AddressDto } from '@/api/admin-wallet'

export function makeAddress(overrides?: Partial<AddressDto>): AddressDto {
	return {
		index: 0,
		address: 'bc1qtest000000000000',
		isUsed: false,
		...overrides,
	}
}
