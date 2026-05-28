import type { PendingBlockPayoutTx, PastBlockPayoutTx } from './block-payouts.types'

const now = new Date()

const hoursFromNow = (h: number) => new Date(now.getTime() + h * 60 * 60 * 1000)
const daysAgo = (d: number) => new Date(now.getTime() - d * 24 * 60 * 60 * 1000)

// Shared outpoint — triggers conflict banner + ⓘ icon
const SHARED_OUTPOINT = 'aabb1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab:2'

export const MOCK_PENDING_TXS: PendingBlockPayoutTx[] = [
	{
		id: 'bp-pending-001',
		inputs: [
			{ outpoint: SHARED_OUTPOINT, amount: 250000, claimId: 'claim-alpha-001', isConflicting: true },
			{
				outpoint: 'ccdd5555aaaabbbb0000111122223333444455556666777788889999aaaabbbb:0',
				amount: 180000,
				claimId: 'claim-alpha-002',
				isConflicting: false,
			},
		],
		signaturesReceived: 1,
		signaturesRequired: 3,
		signedByCurrentUser: false,
		expiresAt: hoursFromNow(72),
		createdAt: daysAgo(1),
		rawTx:
			'0200000001aabb1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab0200000000ffffffff0100e1f50500000000160014abcdef1234567890abcdef1234567890abcdef1200000000',
		signatures: [
			'3044022079be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798022028f800b0ed03a3c0d6e5f1f3c2b4a1e6d7f8c9b0a1b2c3d4e5f60102030405060708',
		],
	},
	{
		id: 'bp-pending-002',
		inputs: [
			{ outpoint: SHARED_OUTPOINT, amount: 250000, claimId: 'claim-beta-001', isConflicting: true },
			{
				outpoint: 'eeff9999ddddeeee1111222233334444555566667777888899990000aaaabbbb:1',
				amount: 320000,
				claimId: 'claim-beta-002',
				isConflicting: false,
			},
			{
				outpoint: '1122334455667788990011223344556677889900112233445566778899001122:3',
				amount: 150000,
				claimId: 'claim-beta-003',
				isConflicting: false,
			},
		],
		signaturesReceived: 2,
		signaturesRequired: 3,
		signedByCurrentUser: false,
		expiresAt: hoursFromNow(48),
		createdAt: daysAgo(2),
		rawTx:
			'0200000003eeff9999ddddeeee1111222233334444555566667777888899990000aaaabbbb0100000000ffffffff0280f0fa0200000000160014deadbeef1234567890abcdef1234567890abcdef00000000',
		signatures: [
			'30440220deadbeef0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c022060f100a1b2c3d4e5f60708090a0b0c0d0e0f101112131415161718191a1b1c1d',
			'304402207890abcdef1234567890abcdef1234567890abcdef1234567890abcdef12345602207890abcdef1234567890abcdef1234567890abcdef1234567890abcdef123456',
		],
	},
	{
		id: 'bp-pending-003',
		inputs: [
			{
				outpoint: 'abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234:0',
				amount: 500000,
				claimId: 'claim-gamma-001',
				isConflicting: false,
			},
		],
		signaturesReceived: 3,
		signaturesRequired: 3,
		signedByCurrentUser: true,
		expiresAt: hoursFromNow(90),
		createdAt: daysAgo(1),
		rawTx:
			'0200000001abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd12340000000000ffffffff0120a10700000000001600147890abcdef1234567890abcdef1234567890abcd00000000',
		signatures: [
			'304402201234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef0220fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210',
			'3044022056789abcdef0123456789abcdef0123456789abcdef0123456789abcdef01234022056789abcdef0123456789abcdef0123456789abcdef0123456789abcdef01234',
			'30440220aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899022011223344556677889900aabbccddeeff11223344556677889900aabbccddeeff',
		],
	},
	{
		id: 'bp-pending-004',
		inputs: [
			{
				outpoint: 'ffff0000eeee1111dddd2222cccc3333bbbb4444aaaa5555999966667777aaab:1',
				amount: 75000,
				claimId: 'claim-delta-001',
				isConflicting: false,
			},
			{
				outpoint: '9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba:0',
				amount: 95000,
				claimId: 'claim-delta-002',
				isConflicting: false,
			},
		],
		signaturesReceived: 0,
		signaturesRequired: 3,
		signedByCurrentUser: false,
		expiresAt: hoursFromNow(3),
		createdAt: daysAgo(4),
		rawTx: '02000000029876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba0100000000ffffffff00000000',
		signatures: [],
	},
]

export const MOCK_PAST_TXS: PastBlockPayoutTx[] = [
	{
		id: 'bp-past-001',
		status: 'unconfirmed',
		broadcastAt: daysAgo(0.5),
		rawTx:
			'0200000002aaaa1111bbbb2222cccc3333dddd4444eeee5555ffff6666000011112222333300000000ffffffff0180380100000000001600145678901234567890123456789012345678901234ffffffff00000000',
	},
	{
		id: 'bp-past-002',
		status: 'confirmed',
		broadcastAt: daysAgo(3),
		blockTimestamp: daysAgo(2.9),
		rawTx:
			'02000000011234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef0000000000ffffffff0140420f00000000001600149012345678901234567890123456789012345678ffffffff00000000',
	},
]
