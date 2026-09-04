import assert from 'node:assert/strict'
import { z } from 'zod'
import { proposalSchema } from './ipc-schemas.ts'

const proposalWithNullBroadcastFields = {
	actionId: 'test-action-id',
	seqNo: 1,
	authority: 'strata_admin',
	status: 'pending',
	requiredSignatures: 2,
	actionHex: '0x01',
	actionType: 'multisig_update',
	signatures: [],
	broadcastStatus: 'idle',
	commitTxid: null,
	revealTxid: null,
	broadcastError: null,
	isCancelable: false,
	createdAtMs: 1000000,
	updatedAtMs: 1000000,
	expiresAtMs: 2000000,
}

const parsed = proposalSchema.safeParse(proposalWithNullBroadcastFields)
assert.equal(parsed.success, true, `expected parse success, got: ${parsed.success ? '' : parsed.error.message}`)
if (parsed.success) {
	assert.equal(parsed.data.commitTxid, undefined)
	assert.equal(parsed.data.revealTxid, undefined)
	assert.equal(parsed.data.broadcastError, undefined)
}

import { AuthRole } from '../types/auth-role.ts'
import { authChallengeSchema, authSessionSchema } from './ipc-schemas.ts'

const challenge = {
	challengeId: 'c1',
	challengeHex: 'aa',
	challengeMessage: 'Strata Session Authentication v1 | Role: strata_admin | Challenge: aa',
	nonceHex: 'bb',
	domain: 'alpen-multisig',
	role: 'strata_administrator',
	issuedAtUnixMs: 1,
	expiresAtUnixMs: 2,
	sessionId: 's1',
}
const challengeParsed = authChallengeSchema.safeParse(challenge)
assert.equal(challengeParsed.success, true)
if (challengeParsed.success) {
	assert.equal(challengeParsed.data.role, AuthRole.StrataAdministrator)
}

const session = {
	role: 'strata_administrator',
	signerPubkeyHex: '02' + '00'.repeat(32),
	authenticatedAtUnixMs: 1,
	expiresAtUnixMs: 2,
	membershipFetchedAtUnixMs: 1,
}
const sessionParsed = authSessionSchema.safeParse(session)
assert.equal(sessionParsed.success, true)
if (sessionParsed.success) {
	assert.equal(sessionParsed.data.role, AuthRole.StrataAdministrator)
}

console.log('ipc-schemas: proposal null Option fields OK')
console.log('ipc-schemas: auth schemas OK')

import {
	sighashResultSchema,
	verifyResultSchema,
	signatureResultSchema,
	decodedActionSchema,
	rawOrchestratorAuthChallengeSchema,
	rawOrchestratorAuthSessionSchema,
	multisigConfigSchema,
	authorityMembershipsSchema,
	buildActionHexResponseSchema,
} from './ipc-schemas.ts'

// signing schemas
const sighash = sighashResultSchema.safeParse({ sighashHex: 'aabb', seqno: 1 })
assert.equal(sighash.success, true)

const verify = verifyResultSchema.safeParse({ valid: true, signaturesVerified: 2, thresholdRequired: 2 })
assert.equal(verify.success, true)

const sig = signatureResultSchema.safeParse({ publicKeyHex: '02aa', signatureHex: 'cc' })
assert.equal(sig.success, true)

const actionMultisig = decodedActionSchema.safeParse({
	kind: 'multisig_update',
	role: 'strata_admin',
	addKeys: ['02aa'],
	removeKeys: [],
	newThreshold: 2,
})
assert.equal(actionMultisig.success, true)

const actionUnknown = decodedActionSchema.safeParse({ kind: 'unknown', rawHex: 'deadbeef' })
assert.equal(actionUnknown.success, true)

const actionVk = decodedActionSchema.safeParse({
	kind: 'vk_update',
	authority: 'strata_admin',
	typeId: 1,
	conditionHex: '',
})
assert.equal(actionVk.success, true)

// orchestrator-auth raw schemas (snake_case from Tauri)
const rawChallenge = rawOrchestratorAuthChallengeSchema.safeParse({
	challenge_id: 'c1',
	challenge_hex: 'aa',
	challenge_message: 'Sign this message',
})
assert.equal(rawChallenge.success, true)

const rawSession = rawOrchestratorAuthSessionSchema.safeParse({
	token: 'tok',
	authority: 'strata_admin',
	signer_pubkey: '02aa',
	expires_at_unix_ms: 9999,
})
assert.equal(rawSession.success, true)

// asm-state schemas
const config = multisigConfigSchema.safeParse({ signers: ['02aa', '02bb'], threshold: 2 })
assert.equal(config.success, true)

const memberships = authorityMembershipsSchema.safeParse({ strata_admin: true, sequencer_manager: false })
assert.equal(memberships.success, true)

// action-builder schema
const buildResp = buildActionHexResponseSchema.safeParse({ actionHex: 'deadbeef' })
assert.equal(buildResp.success, true)

console.log('ipc-schemas: signing schemas OK')
console.log('ipc-schemas: orchestrator-auth raw schemas OK')
console.log('ipc-schemas: asm-state schemas OK')
console.log('ipc-schemas: action-builder schema OK')

// P-023: ApiResult type carries errorCode on failure
// Verified by TypeScript compilation: ApiResult<T> now includes errorCode?: string
// tauriCall sets errorCode: 'ipc_schema_mismatch' on Zod parse failure (branch in tauri-bridge.ts)
import type { ApiResult } from '../types/index.ts'
const _typeCheck: ApiResult<number> = { ok: false, error: 'test', errorCode: 'ipc_schema_mismatch' }
assert.equal(_typeCheck.ok, false)
if (!_typeCheck.ok) {
	assert.equal(_typeCheck.errorCode, 'ipc_schema_mismatch')
}

console.log('ipc-schemas: P-023 errorCode field on ApiResult OK')

// Security Council Phase 5: both IPC boundaries accept Defcon 1. Each is a closed schema, so an
// unregistered value is a parse error rather than an unknown-action fallback — and for
// `actionType` that takes down the parse of every proposal in the same list.
const defcon1Proposal = proposalSchema.safeParse({ ...proposalWithNullBroadcastFields, actionType: 'defcon_1' })
assert.equal(defcon1Proposal.success, true, 'proposalSchema must accept actionType defcon_1')

assert.equal(decodedActionSchema.safeParse({ kind: 'defcon_1' }).success, true)

console.log('ipc-schemas: Defcon 1 boundaries OK')

// Defcon 3 (V2) Phase 1: the same two boundaries, plus the blast radius that made this phase go
// first. `listProposals` parses `z.array(proposalSchema)`, so an unregistered `actionType` does
// not degrade one row — it empties the whole list.
const defcon3Proposal = proposalSchema.safeParse({ ...proposalWithNullBroadcastFields, actionType: 'defcon_3' })
assert.equal(defcon3Proposal.success, true, 'proposalSchema must accept actionType defcon_3')

assert.equal(decodedActionSchema.safeParse({ kind: 'defcon_3' }).success, true)

const mixedList = z.array(proposalSchema).safeParse([
	{ ...proposalWithNullBroadcastFields, actionType: 'defcon_1' },
	{ ...proposalWithNullBroadcastFields, actionType: 'defcon_3' },
])
assert.equal(mixedList.success, true, 'one defcon_3 row must not take down the list beside it')

console.log('ipc-schemas: Defcon 3 boundaries OK')

const withoutCancelable = { ...proposalWithNullBroadcastFields }
delete (withoutCancelable as { isCancelable?: boolean }).isCancelable
assert.equal(proposalSchema.safeParse(withoutCancelable).success, false, 'proposalSchema must require isCancelable')

console.log('ipc-schemas: isCancelable field OK')

// Defcon 3 (V2) Phase 7: `decodedActionSchema` gains the `cancel` member, closing the gap the
// emitter side (action_builder.rs) requires — a Tauri emitting `kind: 'cancel'` against a schema
// that rejects it would fail the parse.
assert.equal(
	decodedActionSchema.safeParse({ kind: 'cancel', targetUpdateId: 7, targetActionHex: 'ab' }).success,
	true,
	'decodedActionSchema must accept a well-formed cancel member',
)

console.log('ipc-schemas: cancel decoded-action member OK')
