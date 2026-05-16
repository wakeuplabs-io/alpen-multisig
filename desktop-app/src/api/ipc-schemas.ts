import { z } from 'zod'

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
