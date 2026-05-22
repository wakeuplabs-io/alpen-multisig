# Diverge / Options Coherence — Adversarial Assessment

**Assessment date:** 2026-05-14  
**Mode:** Read-only adversarial audit  
**Scope:** Implicit dual paths (backend vs desktop, online vs promised manual fallback), feature flags, duplicated domain models, and undocumented trade-offs that should be ADRs.

---

## 1. Scope & threat model

**Vectors**

1. **Parallel implementations** — Same concern (errors, ASM reads, codec, authority enum) solved twice without a recorded decision.
2. **Silent defaults** — Configuration or mock detection that behaves differently in “looks like prod” setups.
3. **ADR debt** — Important forks (e.g., whether desktop ever validates protocol rules) left as tribal knowledge.
4. **Unequal test surfaces** — One path heavily tested (e.g., Strata Admin + localhost mocks), others untested.

---

## 2. Top findings (ranked)

### Blocking / high

**1. Typed `AppError` (backend) vs `String` errors (desktop Tauri layer)**

- **Risk:** UI cannot discriminate session expiry, network failure, or device disconnect without brittle string matching; retries and copy are inconsistent.
- **Evidence:**
  - `orchestrator-be/src/error.rs` — Structured `AppError` with HTTP mapping.
  - Application modules under `desktop-app/src-tauri/src/application/` — pervasive `Result<_, String>` pattern (e.g., proposals, auth flows).
- **Adversarial take:** Two stacks intentionally “know” the right pattern; divergence is undocumented as a product/architecture choice.

**2. “Single codec module” invariant contradicted by signing implementation**

- **Risk:** Protocol-facing imports scatter; upgrades to `strata_asm_*` require wide edits; reviews miss new import sites.
- **Evidence:**

```1:5:desktop-app/src-tauri/src/infrastructure/action_codec.rs
//! Codec between the client domain `Action` and the Strata-owned `MultisigAction`
//! SSZ form.
//!
//! This is the **only** module that imports `strata_asm_*` / `strata_crypto` crates.
//! Everything else in the desktop application talks in domain types.
```

```8:11:desktop-app/src-tauri/src/infrastructure/signing.rs
use ssz::Decode;
use strata_asm_txs_admin::actions::MultisigAction;
use strata_asm_txs_admin::signing_message::SigningMessage;
```

- **Adversarial take:** Comment and reality diverge — either enforce or rewrite the invariant.

**3. Duplicate `Authority` enums (backend + desktop)**

- **Risk:** Drift when Alpen adds or renames roles; one side compiles while the other mis-serializes IPC or REST payloads.
- **Evidence:** `orchestrator-be/src/domain/authority.rs`, `desktop-app/src-tauri/src/domain/authority.rs`; codec bridge in `desktop-app/src-tauri/src/infrastructure/action_codec.rs`.
- **Cross-check:** `docs/architecture/adrs/005-layered-architecture.md` discusses layering; **no ADR** titled “shared types” or “authority SSOT” in `docs/architecture/adrs/` (only 001–005 present).

### Medium

**4. ASM mock strategy (URL substring / localhost) vs repository DI**

- **Risk:** Accidental mock activation or flaky CI when RPC URLs resemble dev patterns.
- **Evidence:** `orchestrator-be/src/infrastructure/asm_role_membership.rs`, `desktop-app/src-tauri/src/infrastructure/asm_role_membership.rs` — mock helpers tied to URL heuristics; desktop mock coverage historically thinner for non–Strata Admin paths (verify when changing tests).

**5. Config defaults for secrets / magic bytes**

- **Risk:** Process accident runs coordination with deterministic test material.
- **Evidence:** `orchestrator-be/src/config.rs` — `OPERATOR_SECRET_KEY_HEX` and `BITCOIN_MAGIC_BYTES_HEX` fallback patterns (review env deployment checklist against code).

**6. Handler thinness inconsistent**

- **Risk:** Validation split between `handlers/` and `application/` without a written rule; contributors guess wrong.
- **Evidence:** Compare `orchestrator-be/src/handlers/auth.rs` vs `handlers/proposals.rs` — mixed inline vs delegated validation styles.

### Low

**7. vestigial or under-explained Cargo features (e.g., Tauri `custom-protocol`)**

- **Risk:** Security or packaging assumptions that do not match CI.
- **Evidence:** `desktop-app/src-tauri/Cargo.toml` `[features]` — verify actual `cfg(feature = ...)` usage in tree.

---

## 3. Attack narratives (3–6)

### N1: Product demands structured error UX in the desktop shell

Engineers cannot implement “Reconnect Trezor” vs “Session expired” without refactoring every `Result<_, String>` return path. Schedule slips; shippers resort to substring hacks.

### N2: Alpen bumps `MultisigAction` SSZ

Both `action_codec.rs` and `signing.rs` change. One PR updates one file; staging branch passes partial builds until release — merge pain attributed to “upstream flake” instead of boundary violation.

### N3: Sixth authority arrives upstream

Backend enum updated; desktop forgotten. IPC accepts unknown discriminant or fails at runtime for half the team. No ADR required shared-types extraction; grep was the spec.

### N4: Staging URL contains `127.0.0.1` tunnel

ASM mock path activates in an unintended environment; quorum/threshold reads are fiction. Incident is attributed to “bad data” because mock strategy was invisible in ops docs.

### N5: New engineer reads ADR-003 only

Misses that signing still imports Strata crates directly; “follow action_codec-only rule” is wrong. They add a third import site.

### N6: Offline/manual path promised in PRD

Code paths remain orchestrator-only. No flag, no module, no ADR saying “we defer offline.” Security review assumes manual path exists because product doc says so.

---

## 4. Evidence index (paths)

| Topic | Path |
|------|------|
| Backend errors | `orchestrator-be/src/error.rs`, `orchestrator-be/src/handlers/*.rs` |
| Desktop application layer | `desktop-app/src-tauri/src/application/*.rs` |
| Codec comment + imports | `desktop-app/src-tauri/src/infrastructure/action_codec.rs` |
| Signing + SSZ | `desktop-app/src-tauri/src/infrastructure/signing.rs` |
| ASM membership / mocks | `orchestrator-be/src/infrastructure/asm_role_membership.rs`, `desktop-app/src-tauri/src/infrastructure/asm_role_membership.rs` |
| Authority enum | `orchestrator-be/src/domain/authority.rs`, `desktop-app/src-tauri/src/domain/authority.rs` |
| Config defaults | `orchestrator-be/src/config.rs` |
| ADRs (existing 001–005) | `docs/architecture/adrs/` |
| Architecture overview | `docs/architecture/overview.md` |
| PRD coordination / fallback | `docs/0-prd/02-multisig-backend.md` |

---

## 5. Smallest fixes vs. largest bets

**Smallest**

- Fix the action_codec header comment **or** route all `MultisigAction` decode/signing through `action_codec` wrappers (single edit surface).
- Document in an existing ADR appendix: **handler vs application validation** boundaries (one paragraph + example).
- Add `Config::validate()` notes to ops-facing doc when that doc exists; until then, inline README warning for operator key.

**Largest**

- Introduce `DesktopError` (or similar) and migrate Tauri command surfaces to typed errors with stable `serde` representation.
- Extract shared `Authority` / proposal identifiers crate consumed by `orchestrator-be` and `src-tauri`.
- Replace URL-string mocks with injected `AsmState` traits in both processes; align e2e fixtures.
- Author **ADR-006+** topics: backend uptime assumption, offline scope, error strategy parity.

---

## 6. What would change my mind

- A **written ADR** “Why desktop uses `String` errors” with IPC/schema rationale would downgrade finding 1 from “high” to “accepted debt.”
- A **compile-time or CI guard** (test that greps for forbidden `strata_asm` import sites outside `action_codec.rs`) would neutralize the boundary violation as a process issue.
- Evidence that **all five authorities** are covered by the same mock + e2e matrix would soften mock-strategy findings.
