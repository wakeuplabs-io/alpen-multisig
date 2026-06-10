import { z } from 'zod'

import { AuthRole } from '@/types/auth-role'

/** Reusable schema combinator: accepts null or undefined, transforms to undefined. */
function nullishToUndefined<T extends z.ZodType>(schema: T) {
	return schema.nullish().transform((v) => v ?? undefined)
}

/** Reusable schema combinator: accepts null or undefined, transforms to null. */
function nullishToNull<T extends z.ZodType>(schema: T) {
	return schema.nullish().transform((v) => v ?? null)
}

export const proposalStatusSchema = z.enum(['pending', 'approved', 'enacted', 'canceled', 'expired'])

export const broadcastStatusSchema = z.enum([
	'idle',
	'commit_broadcasted',
	'commit_confirmed',
	'reveal_broadcasted',
	'reveal_confirmed',
	'failed',
])

const cancelProposalSummarySchema = z.object({
	actionId: z.string(),
	status: proposalStatusSchema,
	signatures: z.array(
		z.object({
			signerPubkey: z.string(),
			signatureHex: z.string(),
		}),
	),
	requiredSignatures: z.number(),
})

export const proposalSchema = z
	.object({
		actionId: z.string(),
		seqNo: z.number(),
		authority: z.string(),
		status: proposalStatusSchema,
		requiredSignatures: z.number(),
		actionHex: z.string(),
		actionType: z.enum(['multisig_update', 'vk_update', 'operator_set_update', 'cancel', 'unknown']),
		signatures: z.array(
			z.object({
				signerPubkey: z.string(),
				signatureHex: z.string(),
			}),
		),
		broadcastStatus: broadcastStatusSchema,
		// Tauri/serde emits null for Option::None — .optional() alone rejects null (P-008).
		commitTxid: nullishToUndefined(z.string()),
		revealTxid: nullishToUndefined(z.string()),
		broadcastError: nullishToUndefined(z.string()),
		targetActionId: nullishToNull(z.string()),
		activationHeight: nullishToNull(z.number()),
		updateIdInQueue: nullishToNull(z.number()),
		cancelProposal: nullishToNull(cancelProposalSummarySchema),
		createdAtMs: z.number(),
		expiresAtMs: z.number(),
	})
	.transform((p) => ({ ...p, kind: p.targetActionId !== null ? ('cancel' as const) : ('update' as const) }))

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
	session: nullishToNull(authSessionSchema),
})

export const broadcastResultSchema = z.object({
	actionId: z.string(),
	proposalStatus: proposalStatusSchema,
	broadcastStatus: broadcastStatusSchema,
	commitTxid: nullishToUndefined(z.string()),
	revealTxid: nullishToUndefined(z.string()),
})

// signing.ts schemas
export const sighashResultSchema = z.object({
	sighashHex: z.string(),
	seqno: z.number(),
})

export const verifyResultSchema = z.object({
	valid: z.boolean(),
	signaturesVerified: z.number(),
	thresholdRequired: z.number(),
})

export const signatureResultSchema = z.object({
	publicKeyHex: z.string(),
	signatureHex: z.string(),
})

export const decodedActionSchema = z.discriminatedUnion('kind', [
	z.object({
		kind: z.literal('multisig_update'),
		role: z.string(),
		addKeys: z.array(z.string()),
		removeKeys: z.array(z.string()),
		newThreshold: z.number(),
	}),
	z.object({ kind: z.literal('unknown'), rawHex: z.string() }),
])

// orchestrator-auth.ts raw schemas (Tauri emits snake_case for these commands)
export const rawOrchestratorAuthChallengeSchema = z.object({
	challenge_id: z.string(),
	challenge_hex: z.string(),
})

export const rawOrchestratorAuthSessionSchema = z.object({
	token: z.string(),
	authority: z.string(),
	signer_pubkey: z.string(),
	expires_at_unix_ms: z.number(),
})

// asm-state.ts schemas
export const multisigConfigSchema = z.object({
	signers: z.array(z.string()),
	threshold: z.number(),
})

export const currentVkSchema = z.object({
	typeId: z.number(),
	typeName: z.string(),
	conditionHex: z.string(),
})

export const authorityMembershipsSchema = z.record(z.string(), z.boolean())

// action-builder.ts schema
export const buildActionHexResponseSchema = z.object({
	actionHex: z.string(),
})

// admin-wallet capability DTO — evolved from bare-bool to {canSign, signerKind, reason?}
export const signerKindSchema = z.enum(['hardware', 'mnemonic', 'none'])

/** Normalizes backend device labels and legacy values to the FE capability union. */
export function normalizeCapabilitySignerKind(raw: string): z.infer<typeof signerKindSchema> {
	if (raw === 'trezor' || raw === 'ledger' || raw === 'hardware') {
		return 'hardware'
	}
	if (raw === 'mnemonic') {
		return 'mnemonic'
	}
	return 'none'
}

export const adminWalletCapabilitySchema = z.union([
	z
		.object({
			canSign: z.boolean(),
			signerKind: z.string(),
			reason: z.string().nullish(),
		})
		.transform((d) => ({
			canSign: d.canSign,
			signerKind: normalizeCapabilitySignerKind(d.signerKind),
			reason: d.reason ?? undefined,
		})),
	z.boolean().transform((b) => ({
		canSign: b,
		signerKind: 'none' as const,
		reason: undefined,
	})),
])

// node-config.ts schemas
export const connectionModeSchema = z.enum(['local', 'trusted', 'custom'])

export const nodeConfigSchema = z.object({
	mode: connectionModeSchema,
	customStrataRpcUrl: z
		.string()
		.nullish()
		.transform((v) => v ?? undefined),
	customBtcRpcUrl: z
		.string()
		.nullish()
		.transform((v) => v ?? undefined),
	customBtcRpcUser: z
		.string()
		.nullish()
		.transform((v) => v ?? undefined),
	customBtcRpcPass: z
		.string()
		.nullish()
		.transform((v) => v ?? undefined),
	customElectrumUrl: z
		.string()
		.nullish()
		.transform((v) => v ?? undefined),
})

export const localNodeStatusSchema = z.object({
	strataReachable: z.boolean(),
	btcReachable: z.boolean(),
	electrumReachable: z.boolean(),
})
