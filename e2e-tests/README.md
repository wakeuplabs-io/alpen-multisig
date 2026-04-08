# E2E Tests — Alpen Multisig

End-to-end integration tests that validate the Alpen/Strata crate APIs work correctly for the multisig admin flows. See [discovery findings](../docs/2-discovery/03-poc1-findings.md) for full context.

## What it tests

The `e2e_admin_subprotocol` test walks through the full admin action flow, stopping just before broadcast:

1. **Generate signer keys** (3 signers, threshold 2-of-3) — simulates hardware wallets
2. **Build a MultisigAction** — a Strata Administrator signer set update (add a new key)
3. **Compute the sighash** — SPS-65 tagged hash: `SHA256(SHA256(tag) || seqno || payload)`
4. **Sign with ECDSA** — 2 of 3 signers sign (reaching threshold)
5. **Build the Bitcoin transaction** — SPS-50 OP_RETURN tag + SPS-51 witness envelope
6. **Parse back and verify** — simulates what the ASM does: extract `SignedPayload`, verify threshold signatures
7. **Broadcast (skipped)** — commented with instructions for production usage

## Requirements

- **Rust nightly-2026-01-01** (installed automatically via `rust-toolchain.toml`)
- Internet access on first build (fetches git dependencies from `alpenlabs/alpen` and `alpenlabs/strata-common`)

## Run

```sh
cargo test
```
