# P-053 — Digest verification usability test

## Hypothesis

Signers can match a displayed 64-char sighash to Trezor screen with &lt;2 errors when authority label is visible.

## Protocol

1. Show authority name + sighash hex on desktop preview (P-006 gate).
2. Participant compares to HW device.
3. Measure: time to confirm, mis-clicks, request for help.

## Success criteria

- ≥80% complete without requesting “what am I signing?”
- Document failure modes for P-006 UX copy

## Manual-fallback tabletop (paired)

Scenario: orchestrator stopped after `claim_broadcast`; desktop still has signatures.

Steps: export bundle → local `sendrawtransaction` → document metadata catch-up when coordinator returns.
