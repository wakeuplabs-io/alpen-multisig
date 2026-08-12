# ADR-007: ASM pin target for Security Council

**Status:** Accepted
**Date:** 2026-08-12
**Extends:** [ADR-001](./001-alpen-crate-dependencies.md) — this is an application of its pin-update procedure, not a change to it.
**Feature:** [`specs/security-council.md`](../../specs/security-council.md)

## Context

The Security Council feature cannot be built at our current pin. `Role::StrataSecurityCouncil`,
`Defcon1`, `Defcon3`, `SafeHarbourAddressUpdate` and `StrataSecurityCouncilMultisigUpdate` all
arrived in `alpenlabs/asm` PR #81 (merge commit `3d45351`, 2026-05-30). We are pinned at `e0461f8`
(2026-05-11), 19 commits earlier.

So the pin has to move. The question is how far, and that question has a real cost attached: the
bump is wire-format-breaking (`StrataSecurityCouncilMultisig` is inserted at SSZ union selector 3,
shifting `OperatorSet` and every later variant), so it has to be done once, deliberately, with an
operational reset — not incrementally.

We evaluated four candidates by spiking each one in a throwaway worktree off `develop`, moving all
four Cargo files that carry the pin (`Cargo.toml`, `e2e-tests/Cargo.toml`,
`orchestrator-be/Cargo.toml`, `desktop-app/src-tauri/Cargo.toml`) plus the transitive
`strata-common` and `ssz-gen` tags, then building.

| Candidate | Date | Has PR #81 | Crate names survive | strata-common | ssz-gen | Bitcoin Core |
|---|---|---|---|---|---|---|
| `e0461f8` (current) | 2026-05-11 | no | — | rc19 | v0.15.0 | 29.0 |
| **`v0.1-alpha.11`** | 2026-06-02 | **yes** (first tag) | **yes, 13/13** | rc23 | v0.16.0 | **29.0** |
| `v0.1-alpha.14` | 2026-06-17 | yes | yes | rc26 | v0.16.0 | 30.2 |
| `v0.3.1` | 2026-08-11 | yes | yes, 13/13 | v0.3.0 | v0.17.0 | 30.2 |
| `v0.4.0-rc.1` / `main` | 2026-08 | yes | **no** | v0.3.0 | v0.17.0 | 30.2 |

## Decision

**Pin to `v0.1-alpha.11` (`b84eb28a71b99ed54e128ca282ed8f637c2e88ef`)**, with `strata-common`
`v0.1.0-alpha-rc23` and `ssz-gen` `v0.16.0`.

Three findings drove this, in order of weight.

### 1. Bitcoin Core 29.0 → 30.2 (decisive)

`v0.1-alpha.11` is the **only** tag containing PR #81 that still targets Bitcoin Core 29.0. From
`v0.1-alpha.12` onward upstream moved to 30.2 (`bitcoind-async-client` feature `30_2`,
`corepc-node 0.12`). Those features are mutually exclusive, so this is a hard build failure:

```
error: Bitcoin Core version features are mutually exclusive; select only one of `29_0` or `30_2`.
```

A Core 30.2 bump would drag in, none of it related to Security Council:
`.github/workflows/ci.yml:93` (`BITCOIN_CORE_VERSION: "29.0"`, with a comment tying it to the
workspace features), `staging/Dockerfile.bitcoin:16` (still on 28.1), `corepc-node` 0.10 → 0.12 in
the e2e harness, and every developer's local `bitcoind` — where the e2e tests skip themselves when
the binary is missing, so a mismatch degrades into silently-omitted coverage rather than a red
build.

### 2. Pinning newer buys no feature surface

The entire role/action/segregation model is **byte-identical** between `v0.1-alpha.11` and
`v0.3.1`: roles, all twelve `UpdateTxType` variants and their bytes, `authorized_role()`,
`ConfirmationDepths` including the hardcoded Defcon 1 depth of 0, `AdministrationInitConfig`,
`MultisigAction`, `UpdateAction` and its union order, all four council-relevant action types,
`CancelAction`, and `SigningMessage`. PR #81 landed the model complete and nothing since has
changed it. Pinning newer costs a Core migration and buys nothing this feature needs.

### 3. The compile-break surface at `v0.1-alpha.11` is four mechanical fixes

Measured by iterating `cargo build --workspace --all-targets` to green:

1. `strata_asm_proto_checkpoint::{state, subprotocol}` became flat re-exports — three files.
2. Non-exhaustive match on `MultisigAction` at
   `desktop-app/src-tauri/src/infrastructure/action_codec.rs:188` for the four new variants.
3. `WorkerContext::has_l1_manifest` no longer exists on the trait;
   `e2e-tests/src/worker_context.rs` implements it.
4. `AdministrationInitConfig` gains a required `strata_security_council` field and
   `ConfirmationDepths` gains three (`strata_security_council_multisig_update`, `defcon3`,
   `safe_harbour_address_update`) — `e2e-tests/tests/e2e_cancel_proposal.rs` and the inline JSON
   fixtures in `e2e-tests/src/fixtures/signer_update_enacted.rs`.

`v0.1-alpha.11` still requires `bitcoind-async-client 0.10.1`, so our existing `=0.10.6` pin and
its `29_0` feature stand unchanged. `v0.3.1` never reached a compile at all: it needs `^0.10.8`,
and every version satisfying that carries the `30_2` feature, which collides with ours during
dependency resolution.

## Consequences

- **The Core 30.2 migration becomes separate, sequenced work.** It is a CI + staging + local
  toolchain change that should not land inside the gate that answers "is the ASM ready for
  Security Council?". Recorded as follow-up, not dropped.
- **We knowingly pin behind upstream HEAD.** At the time of writing that is ~96 commits and two
  minor lines. Acceptable because the feature surface is identical and ADR-001's procedure exists
  precisely to make the next move deliberate.
- **`orchestrator-be`'s `action_codec.rs` did not break at this bump**, because it routes unknown
  variants through a `_ =>` catch-all. That is the failure mode we are trying to avoid: it silently
  accepts the four new variants instead of forcing a decision. Every exhaustive match over
  `UpdateAction` / `UpdateTxType` gains **explicit arms** returning a typed "unsupported" error, so
  the next bump fails to compile rather than failing silently.
- **Two runtime breaks the compiler cannot catch.** `AdministrationInitConfig` and
  `ConfirmationDepths` derive `Deserialize` with no `#[serde(default)]`, so any JSON missing the
  four new fields fails to deserialize at runtime: the inline fixtures in
  `e2e-tests/src/fixtures/signer_update_enacted.rs` and `staging/asm-params.template.json`. The
  spike demonstrated this — with the workspace building green and four of five e2e suites passing,
  `e2e_enactment_predicate` still failed at:

  ```
  panicked at e2e-tests/src/fixtures/signer_update_enacted.rs:142:
  admin section deserializes into AdministrationInitConfig:
  Error("missing field `strata_security_council_multisig_update`")
  ```

  A green build does not prove these are fixed; the Stage 3 gate does.
- **The wire break is mandatory work, not optional.** Any `action_hex` persisted before the bump
  decodes to a different action after it, and `ActionId = hash(MultisigAction, SeqNo)` values are
  not comparable across the boundary. The Stage 3 operational reset covers the orchestrator
  database, the `strata-asm-runner` binary and its DB, and the regtest datadir.

## Update procedure

Unchanged from ADR-001, with one addition this spike surfaced: the pin lives in **five** places,
not four, and they must move together — the three crate manifests, `e2e-tests/Cargo.toml`, and the
`asm` submodule gitlink.
