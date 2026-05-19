import { z } from 'zod'

import { AuthRole } from '@/types/auth-role'

export const proposalStatusSchema = z.enum(['pending', 'approved', 'enacted', 'canceled', 'expired'])

export const broadcastStatusSchema = z.enum([
	'idle',
	'commit_broadcasted',
	'commit_confirmed',
	'reveal_broadcasted',
	'reveal_confirmed',
	'failed',
])

export const proposalSchema = z.object({
	actionId: z.string(),
	seqNo: z.number(),
	authority: z.string(),
	status: proposalStatusSchema,
	requiredSignatures: z.number(),
	actionHex: z.string(),
	signatures: z.array(
		z.object({
			signerPubkey: z.string(),
			signatureHex: z.string(),
		}),
	),
	broadcastStatus: broadcastStatusSchema,
	// Tauri/serde emits null for Option::None — .optional() alone rejects null (P-008).
	commitTxid: z
		.string()
		.nullish()
		.transform((v) => v ?? undefined),
	revealTxid: z
		.string()
		.nullish()
		.transform((v) => v ?? undefined),
	broadcastError: z
		.string()
		.nullish()
		.transform((v) => v ?? undefined),
})

export const authRoleSchema = z.nativeEnum(AuthRole)

export const authChallengeSchema = z.object({
	challengeId: z.string(),
	challengeHex: z.string(),
	nonceHex: z.string(),
	domain: z.string(),
	role: authRoleSchema,
	issuedAtUnixMs: z.number(),
	expiresAtUnixMs: z.number(),
	sessionId: z.string(),
})

export const authSessionSchema = z.object({
	role: authRoleSchema,
	signerPubkeyHex: z.string(),
	authenticatedAtUnixMs: z.number(),
	expiresAtUnixMs: z.number(),
	membershipFetchedAtUnixMs: z.number(),
})

export const authSessionResultSchema = z.object({
	authenticated: z.boolean(),
	session: authSessionSchema.nullish().transform((v) => v ?? null),
})

export const broadcastResultSchema = z.object({
	actionId: z.string(),
	proposalStatus: proposalStatusSchema,
	broadcastStatus: broadcastStatusSchema,
	commitTxid: z
		.string()
		.nullish()
		.transform((v) => v ?? undefined),
	revealTxid: z
		.string()
		.nullish()
		.transform((v) => v ?? undefined),
})
