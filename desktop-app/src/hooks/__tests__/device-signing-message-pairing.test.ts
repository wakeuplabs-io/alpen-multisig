// The signer compares this message against their hardware device, so a message resolved for one
// action must never be returned for another — the window this guard closes is the frame between
// a new render and the effect that would have cleared the old value.

import assert from 'node:assert/strict'
import { messageForInputs, type Resolved } from '../use-device-signing-message.ts'

const resolved: Resolved = { seqno: 5, actionHex: 'aabb', message: 'Sequence: 5', messageHash: 'hash-5' }
const nothing = { message: null, messageHash: null }

assert.deepEqual(messageForInputs(resolved, 5, 'aabb'), { message: 'Sequence: 5', messageHash: 'hash-5' })

assert.deepEqual(messageForInputs(resolved, 6, 'aabb'), nothing, 'a new sequence number must not reuse the old message')
assert.deepEqual(messageForInputs(resolved, 5, 'ccdd'), nothing, 'a new action must not reuse the old message')
assert.deepEqual(messageForInputs(null, 5, 'aabb'), nothing)
assert.deepEqual(messageForInputs(resolved, null, null), nothing)
