// admin-id-certificate — pure model unit test (tsx + node:assert).
//
// PRD 06 §3.c.i. The copy in this module is transcribed from the three normative
// wireframes in docs/0-prd/assets/, so it is pinned here rather than described: a
// reworded help text is a change to a client-approved screen, not a detail.
//
// The copied block is the certificate's whole point — whoever receives it must be able
// to paste line 1 as the message and line 2 as the signature into any verifier and
// recover the Admin ID's public key, deleting nothing.

import assert from 'node:assert/strict'
import {
	CERTIFICATE_TITLE,
	CERTIFICATE_STEP_1_HEADING,
	CERTIFICATE_STEP_1_HELP,
	CERTIFICATE_WAITING,
	CERTIFICATE_SIGNED_CHIP,
	CERTIFICATE_COPIED,
	certificateBlock,
} from '../admin-id-certificate.ts'

const MESSAGE = 'Admin ID: bc1q5lvgztw04yl7addhh63yry2tsuw5vxj9fxadlp'
const SIGNATURE = 'IDAuO/1idzadrzY3OvEt4xBVILLvuImmpePJBFALhj9PO8n2iM2Cm/7+0kGVfMEGmouaDnua1SCRwyIbt3mut08='

// ── The copied block is two lines, in one order, with no labels ──────────────

assert.equal(certificateBlock(MESSAGE, SIGNATURE), `${MESSAGE}\n${SIGNATURE}`)

const lines = certificateBlock(MESSAGE, SIGNATURE).split('\n')
assert.equal(lines.length, 2, 'exactly two lines — anything else needs editing before verifying')
assert.equal(lines[0], MESSAGE, 'line 1 is the signed message verbatim')
assert.equal(lines[1], SIGNATURE, 'line 2 is the signature verbatim')
assert.ok(!lines[1].startsWith('Signature:'), 'no label to strip before verifying')

// The message is the signed bytes: padding it would break verification for the reader.
assert.equal(certificateBlock(`  ${MESSAGE}  `, ` ${SIGNATURE} `), `${MESSAGE}\n${SIGNATURE}`, 'both lines trimmed')

// An unsigned modal has nothing to copy — better to copy nothing than half a certificate.
assert.equal(certificateBlock(MESSAGE, ''), '', 'no signature yet → no block')
assert.equal(certificateBlock('', SIGNATURE), '', 'no message → no block')

// ── Wireframe literals (docs/0-prd/assets/) ──────────────────────────────────

assert.equal(CERTIFICATE_TITLE, 'Generate Admin ID Verification Certificate')
assert.equal(CERTIFICATE_STEP_1_HEADING, 'Step 1. Sign Admin ID')
assert.equal(
	CERTIFICATE_STEP_1_HELP,
	'Click the "Sign" button and confirm the signature on your hardware signer to digitally sign your Admin ID and generate your Admin ID Verification Certificate.',
)
assert.equal(CERTIFICATE_WAITING, 'Waiting for signature to generate Admin ID Verification Certificate...')
assert.equal(CERTIFICATE_SIGNED_CHIP, 'Signed')
assert.equal(CERTIFICATE_COPIED, 'Copied to clipboard')

// The wireframes name no vendor: the same modal serves Trezor, Ledger and a mnemonic
// session, and #24/#18 settled that device-specific wording belongs in lib/device-copy.
for (const literal of [CERTIFICATE_TITLE, CERTIFICATE_STEP_1_HEADING, CERTIFICATE_STEP_1_HELP, CERTIFICATE_WAITING]) {
	assert.ok(!/trezor|ledger/i.test(literal), `certificate copy must stay device-agnostic: ${literal}`)
}

console.log('admin-id-certificate: all assertions passed.')
