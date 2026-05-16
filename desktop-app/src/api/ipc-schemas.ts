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
	commitTxid: z.string().optional(),
	revealTxid: z.string().optional(),
	broadcastError: z.string().optional(),
})

export const broadcastResultSchema = z.object({
	actionId: z.string(),
	proposalStatus: proposalStatusSchema,
	broadcastStatus: broadcastStatusSchema,
	commitTxid: z.string(),
	revealTxid: z.string(),
})
