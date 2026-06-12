# Adversarial review — Phase 5 fee-bump, Alpen Administrator, external docs

**Date:** 2026-06-12  
**Scope:** Commits merged on `develop` via PR [#279](https://github.com/wakeuplabs-io/alpen-multisig/pull/279) (range `2730f72..99508ad`):

| Commit / PR | Summary |
|-------------|---------|
| [#277](https://github.com/wakeuplabs-io/alpen-multisig/pull/277) | Admin Wallet implementation status (R2, Phase 4/5 plan) |
| [#278](https://github.com/wakeuplabs-io/alpen-multisig/pull/278) | `docs/external/` client-facing deliverables |
| `8e60af1` | Alpen Administrator authority support (codec, ASM, UI) |
| [#276](https://github.com/wakeuplabs-io/alpen-multisig/pull/276) | Phase 5: unconfirmed tx list + fee-bump (RBF / governance CPFP) |

**Method:** Read-only code and doc audit (no runtime probes).  
**Purpose:** Capture findings as a **fix backlog** for follow-up PRs. This document is **internal** — not client-facing.

**Related specs:** [`admin-wallet-transactions-fee-bump.md`](../../specs/admin-wallet-transactions-fee-bump.md), [`admin-wallet-prd-compliance.md`](../../specs/admin-wallet-prd-compliance.md), [`deliverables-reorganization.md`](../../specs/deliverables-reorganization.md).

---

## Executive summary

Phase 5 implements a sound **happy-path** design: RBF for plain wallet sends, CPFP for governance commits with a pending pre-signed reveal (R1.0.1). Rust unit tests in `wallet_transactions.rs` are strong.

The merge is **not safe to treat as closed for signer safety or client delivery** until at least **F-001** (volatile `PendingReveals` → accidental RBF on governance commits after app restart) is fixed. In the same merge batch, **`docs/external/` contradicts the code** on Alpen Administrator availability, and PRD §4.3.3 is marked **PASS** without WebDriver E2E for fee-bump.

---

## Ranked fix backlog

Fix IDs are stable handles for PR titles and trackers. **Priority** reflects signer-safety and delivery risk, not implementation effort.

### Tier 0 — Signer safety (BLOCKING before mainnet / honest §4.3.3 PASS)

#### F-001 — Persist or re-derive governance-commit CPFP guard (CRITICAL)

**Finding:** CPFP dispatch depends on in-memory `PendingReveals` (`desktop-app/src-tauri/src/main.rs`). After crash or restart the map is empty. Governance **commits** are built via BDK `build_tx()` (RBF-signaling by default). The fee-bump path then offers **RBF** instead of CPFP (`wallet_transactions.rs` `bump_fee` / `list_unconfirmed_sent_txs`). Replacing the commit orphans the pre-signed reveal (ephemeral envelope key dropped per R1.0.1).

**Evidence:**

- `pending_reveals.rs` — `Arc<Mutex<HashMap>>`, no disk persistence
- `wallet_transactions.rs` — `match pending_commit_to_reveal.get(txid) { Some → CPFP, None → RBF }`
- `wallet_service.rs` `build_and_sign_tx` — no `set_exact_sequence(Sequence::MAX)` on commits

**Suggested fixes (pick one or combine):**

1. **Persist** `PendingReveals` to app data dir; reload on startup until reveal confirms (mirror orchestrator `broadcast_status` reconciliation).
2. **Infer** governance commits from wallet graph + orchestrator pending broadcast state (commit txid known server-side).
3. **Defense in depth:** refuse RBF on any unconfirmed tx that has a known child reveal in the wallet graph or mempool, even without the in-memory map.

**Acceptance criteria:**

- After app restart with commit broadcasted and reveal pending, list shows `isGovernanceCommit: true`, `bumpMethod: cpfp`, Bump disabled for RBF.
- Attempting RBF on a governance commit txid returns `TxNotReplaceable` or `CpfpOutputUnavailable` with a clear message — never broadcasts a replacement commit.
- Regression test: restart simulation (empty `PendingReveals` + commit+reveal in wallet graph) must not expose RBF on the commit.

**Area:** `desktop-app/src-tauri` (`pending_reveals`, `wallet_transactions`, optionally `proposals.rs`).

---

### Tier 1 — High (correctness, delivery, compliance honesty)

#### F-002 — Reconcile `docs/external/` with Alpen Administrator support

**Finding:** PR `8e60af1` enables Alpen Administrator in wallet-connect (`availabilityLabel: 'Available'`) and ASM/codec paths. `docs/external/integration-test-report.md` and `research-assessment.md` still state Alpen Admin is **blocked / pending upstream**.

**Acceptance criteria:**

- External docs describe Alpen Administrator as **supported for signer multisig updates** (with scope limits).
- VK update (`EeStfVk`) limitations documented separately (see F-003).
- No claim of “all five authorities” full E2E where wallet-connect only exposes two roles.
- `docs/external/README.md` “production-ready” wording qualified or aligned with open phases (6–10).

**Area:** `docs/external/`.

---

#### F-003 — Alpen Administrator VK update: enactment or UI gate

**Finding:** UI offers “Verification key update” for `alpen_admin`. Orchestrator `is_proposal_enacted_on_asm` returns `Ok(false)` for `EeStfVk` unconditionally (`orchestrator-be/src/infrastructure/asm_enactment.rs`). Signer updates (`AlpenAdminMultisig`) are wired; VK path is not.

**Acceptance criteria (choose product decision):**

- **Option A:** Implement `EeStfVk` enactment detection for Alpen Admin (if ASM state exposes the predicate).
- **Option B:** Hide or disable VK update for `alpen_admin` in create-proposal UI with explicit “not yet supported” copy.
- Compliance matrix and external docs updated to match.

**Area:** `orchestrator-be`, `desktop-app/src` (create-proposal).

---

#### F-004 — WebDriver E2E for fee-bump (PRD §4.3.3)

**Finding:** `admin-wallet-prd-compliance.md` marks §4.3.3 **PASS**. Existing WebDriver spec (`e2e-webdriver/test/specs/admin-wallet-panel.e2e.js`) covers unconfirmed **balance** only, not pending-tx list or bump flow.

**Acceptance criteria:**

- E2E (or documented manual gate): fund wallet → create unconfirmed send → list in “Pending transactions” → RBF bump → assert new txid / panel refresh.
- Optional second scenario: governance commit pending → CPFP bump (may require harness helper).
- Compliance matrix notes E2E evidence path.

**Area:** `desktop-app/e2e-webdriver/`, `docs/specs/admin-wallet-prd-compliance.md`.

---

#### F-005 — Soften or evidence-bind external security / test claims

**Finding:** `docs/external/security-review-summary.md` states private keys never leave HW; mnemonic regtest path exists. `integration-test-report.md` cites exact pass counts and “100% coverage” without generation procedure.

**Acceptance criteria:**

- Security summary distinguishes **production HW path** vs **dev mnemonic** (`MnemonicPsbtSigner`, regtest/testnet guards).
- Integration report: either auto-generated from CI artifact or qualified as snapshot with date + command to reproduce.
- Remove or footnote unverifiable fixed percentages unless tied to CI output.

**Area:** `docs/external/`.

---

### Tier 2 — Medium (robustness, UX, doc accuracy)

#### F-006 — CPFP child vsize estimate vs extra inputs

**Finding:** `CPFP_CHILD_VSIZE_EST_VBYTES = 111` is fixed. If BDK adds wallet inputs to fund the child fee, realized package rate can fall below the requested rate (comment acknowledges this).

**Acceptance criteria:**

- After building CPFP child PSBT, compute actual vsize (or BDK fee estimate) and verify package rate ≥ requested rate − ε; return `FeeRateTooLow` if not.
- Or document as known limitation in spec + UI (“estimated child fee”).

**Area:** `wallet_transactions.rs`, `bump-fee-form.tsx`.

---

#### F-007 — CPFP anchor: deterministic reveal change output

**Finding:** `build_cpfp_child_psbt` uses `.position(|out| wallet.is_mine(...))` — first wallet-owned output wins.

**Acceptance criteria:**

- Select reveal change output by protocol convention (e.g. vout 1 per `broadcast_tx` layout) or largest wallet-owned output; test with multiple `is_mine` outputs if feasible.

**Area:** `wallet_transactions.rs`.

---

#### F-008 — Pre-bump sync failure UX

**Finding:** `admin_wallet_bump_fee` logs a warning and proceeds if pre-sync fails.

**Acceptance criteria:**

- Surface `SyncFailed` (or tagged warning) to UI when sync fails before bump; optional hard block unless user confirms “proceed with stale state”.
- Clear copy when reveal not in graph: “Sync the wallet and retry”.

**Area:** `admin_wallet.rs`, `format-admin-wallet-error.ts`, bump form.

---

#### F-009 — CPFP row UI when package stats unknown

**Finding:** `composeUnconfirmedTxRows` sets `usesPackageStats` only when `packageFeeSats !== null`, but `bumpMethod` can still be `cpfp`. UI min-rate / estimate can be wrong until reveal syncs.

**Acceptance criteria:**

- When `bumpMethod === 'cpfp'` and package fields null: disable Confirm, show “Sync to load package fee” (or hide Bump until stats available).

**Area:** `compose-unconfirmed-tx-rows.ts`, `unconfirmed-txs-list.tsx`.

---

#### F-010 — PRD compliance: §4.3.3 PASS vs engineering slice

**Finding:** Matrix states engineering slice ≠ automatic PRD PASS at the top, but §4.3.3 is **PASS** while watch-only cannot bump with HW signing and Trezor Admin Wallet PSBT signing is not implemented.

**Acceptance criteria:**

- Reclassify §4.3.3 as **PARTIAL** until F-001 + F-004 close, OR add explicit Notes column listing session/HW preconditions for PASS.
- Align with [`admin-wallet-implementation-plan.md`](../../specs/admin-wallet-implementation-plan.md) Phase 5 ✅ semantics.

**Area:** `docs/specs/admin-wallet-prd-compliance.md`.

---

### Tier 3 — Low (follow-up, polish)

#### F-011 — Fee-bump signer confirmation depth

**Finding:** Bump confirms rate only; no tx preview / HW screen parity with governance broadcast.

**Acceptance criteria:** Defer to Phase 8/9 or add lightweight summary (new fee, method RBF|CPFP, txid) before Confirm; HW path uses same Ledger PSBT flow as commit when applicable.

**Area:** `bump-fee-form.tsx`, future HW work.

---

#### F-012 — `last_seen_secs` ordering edge case

**Finding:** Txs without indexer `last_seen` sort to the end.

**Acceptance criteria:** Document behavior or fall back to txid / insertion order for stable UI.

**Area:** `wallet_transactions.rs` (optional).

---

## What already holds up (do not regress)

- Typed `BumpFeeError` + tagged IPC `{ type, message }` (fixes empty panel on `Disabled`).
- `TxBroadcaster::broadcast_one` + Electrum-first fallback for replacements.
- CPFP design when `PendingReveals` is populated — correct package fee math and tests (`bump_fee_governance_cpfp_*`).
- `AlpenAdminMultisigUpdate` codec roundtrips in `action_codec.rs`.
- Phase 5 commands registered in both production and dev handler sets with runtime capability guards.

---

## Suggested PR sequencing

| Order | Fix ID | Rationale |
|-------|--------|-----------|
| 1 | F-001 | Signer safety — can brick governance broadcast |
| 2 | F-003 | Alpen VK UI/backend mismatch — user-visible correctness |
| 3 | F-002, F-005 | Client delivery coherence |
| 4 | F-004, F-010 | Evidence for PRD §4.3.3 |
| 5 | F-006–F-009 | Robustness and UX |
| 6 | F-011, F-012 | Polish / defer |

---

## Open questions for product / protocol

1. Should `PendingReveals` persistence be authoritative, or should orchestrator `broadcast_status` + commit/reveal txids be the SSOT after restart?
2. Is Alpen `EeStfVk` in scope for this program or should the UI gate it until ASM enactment exists?
3. Is §4.3.3 PASS acceptable for mnemonic regtest only, with PARTIAL until HW bump path ships (Phase 8)?

---

## Changelog

| Date | Change |
|------|--------|
| 2026-06-12 | Initial backlog from adversarial review of PR #279 batch |
