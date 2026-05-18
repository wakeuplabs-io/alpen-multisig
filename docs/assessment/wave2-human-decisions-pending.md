# Wave 2 — decisions pending (human gate)

Per [action-plan-2026-05-14.md](action-plan-2026-05-14.md) §6. **Do not implement blocked items until approved.**

## 1. P-012 / ADR-006 — threshold-detection policy

**Options:**

| Option | Behavior | Trade-off |
|--------|----------|-----------|
| **A — Remove** | Delete auto-transition to `Approved` when `signatures.len() >= required_signatures` in orchestrator | Aligns with PRD §1 “coordination only”; signers/off-chain UI must infer quorum |
| **B — Advisory carve-out** | Keep transition; document in ADR-006; add threshold-resync test vs ASM | Faster UX; must prove stale `required_signatures` cannot mislead broadcast (pairs with P-035) |

**Stakeholders:** Alpen + Wakeup architecture leads.

**Blocked:** Track B `P-012` implementation; ADR-006 final wording.

---

## 2. Operator-key custody (P-001, P-003, P-040)

**Options:** process env at Tauri startup (current + P-001 gate), OS keychain, HSM, hardware-wallet-only operator.

**Blocked:** Track A `P-003` (mnemonic off IPC) and `P-040` (capabilities) design.

**Interim shipped:** P-001 desktop rejects well-known test key unless `ALLOW_DEV_OPERATOR_KEY=1`.

---

## 3. US-H5 manual-fallback scope (P-052, P-053, Track E)

**Question:** Is coordinator-down broadcast (export hex + local RPC) Slice-0 invariant or deferred?

**Blocked:** Track E orchestrator-down WDIO matrix scope.

---

## 4. P-055 — SPS excerpts in repository

**Question:** May we archive SPS-50/51/65 excerpts under `docs/specs/sps-reference/`?

**Stakeholder:** Alpen legal-of-record.

**Blocked:** Track F `P-055` content import.

---

## 5. Production vs test mnemonic path

**Question:** Is mnemonic-over-IPC acceptable only in dev/E2E (`ALLOW_DEV_*`), or must production builds compile it out?

**Blocked:** Track A `P-003` and Track E E2E strategy.
