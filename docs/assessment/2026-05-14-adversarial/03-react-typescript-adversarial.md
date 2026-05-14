# React & TypeScript (desktop-app/src) — Adversarial Assessment

## Scope & threat model (what we're trying to break)

- **Signer safety UX**: Explicit authority context before signing; no ambiguity between wallet-auth vs orchestrator session; truthful success/failure from `invoke` wrappers.
- **`tauri-bridge` normalization**: Errors and successes crossing JS↔Rust must not bury structured backend signals needed for remediation.
- **Session coupling**: Wallet role, orchestrator bearer (in Tauri), and displayed countdown must stay coherent (`contexts/session-provider.tsx`, hooks).
- **Type discipline**: Zod/domain models vs unchecked strings at integration boundaries (`api/proposals.ts` typed unions vs runtime validation).

## Top findings (ranked) — Blocking/High | Medium | Low

### Blocking / High

1. **`authorityFromRole` collapse to `strata_admin`.** `orchestrator-auth.ts` maps `AuthRole.StrataAdministrator` / `StrataSequencerManager` explicitly; **`default` returns `'strata_admin'`** (`desktop-app/src/api/orchestrator-auth.ts`). Any newly added UI role accidentally falls through to Strata Administrator wire authority → wrong signing set / unintended multisig lineage if backend rejects later or accepts wrong authority in mixed deployments.

2. **`BroadcastResult` trust after IPC.** Frontend types `proposalStatus` / `broadcastStatus` as strict unions (`api/proposals.ts`), but Rust `proposals_broadcast` currently returns canned strings (`src-tauri/.../proposals.rs`). React displays “success paths” inconsistent with orchestrator-enums without runtime guard — signer false confidence (**paired finding with Tauri doc**).

3. **`tauriCall` collapses rejection to opaque string.** `tauri-bridge.ts` catches all errors and exposes `error: string`; HTTP status differentiation (401 vs 409) parsed in Rust for some paths never reaches typed discriminated unions in TS — **`ApiResult` lacks error codes**. Risk: generic toast on fatal authority error; user retries blindly.

### Medium

4. **Dual login rituals complexity.** `SessionProvider` chains wallet `authenticate`, then orchestrator challenge with `authorityFromRole(selectedRole)` (`session-provider.tsx`). Desync scenarios: orchestrator succeeds with authority A while user changed role dropdown mid-flow (depends on locking UI — adversarial reviewer assumes race until proven blocked in UI).

5. **`seqNo: number` in `Proposal`** (`api/proposals.ts`). JavaScript doubles lose integer precision beyond `Number.MAX_SAFE_INTEGER`; protocol permits `u64`. Practically multisig sequences stay small — still a **silent future drift** if UI ever echoes huge seqnos.

6. **`ORCHESTRATOR_BASE_URL` defaults to LAN** (`api/orchestrator-auth.ts`): fine for POV; easy misconfiguration pointing at HTTP MITM hotspot without TLS pinning narrative.

### Low

7. **Example in `tauri-bridge.ts` cites wrong command naming** (`list_proposals` vs actual `proposals_list`) — documentation rot, wastes incident time.

## Attack narratives (3–6)

1. **Role mis-bind.** Maintainer adds Security Council UI role but forgets `authorityFromRole` switch → silent default `'strata_admin'` → signer proves membership for unrelated authority wording in UI labels.

2. **Generic error spiral.** Repeated failed approve after backend 409 conflict; UI shows stringified Tauri message; signer assumes hardware fault, re-signs different payload.

3. **Session timer false security.** Wall-clock countdown uses `expiresAtUnixMs` from orchestrator (`session-provider.tsx`); if token revoked server-side only, countdown still ticks — rare but misleading.

## Evidence index (paths)

| Topic | Paths |
|-------|-------|
| Invoke wrapper | `desktop-app/src/api/tauri-bridge.ts` |
| Orchestrator URLs / authority mapping | `desktop-app/src/api/orchestrator-auth.ts` |
| Proposal / broadcast types | `desktop-app/src/api/proposals.ts` |
| Session UX | `desktop-app/src/contexts/session-provider.tsx`, `screens/*` |
| Auth roles definition | `desktop-app/src/types/auth-role.ts` |

## Smallest fixes vs largest bets (be explicit)

**Smallest**

- Make `authorityFromRole` exhaustive (`never` branch) — compile error when `AuthRole` grows.
- Extend `ApiResult` error channel with `{ code, message }` from structured Tauri errors.
- After broadcast, **`getProposalByActionId` refresh** (already pattern elsewhere) instead of trusting `BroadcastResult`.
- Align `seqNo` to `bigint` or string for wire fidelity if protocol demands.

**Largest bets**

- OpenAPI/generated TS client matching orchestrator DTOs; runtime Zod decode on every IPC response.
- Single “signing ceremony” state machine enforcing locked role through dual auth phases.

## What would change my mind (missing evidence / experiments)

- UI proof that role cannot change between wallet auth and orchestrator complete (interaction test).
- Lint rule or test scanning `authorityFromRole` switches for exhaustiveness.
- Product confirmation that only two authorities exist permanently — narrows severity of default branch (still prefer compile-time enforcement).
