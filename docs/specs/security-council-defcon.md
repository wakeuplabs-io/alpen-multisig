# Spec: Security Council Multisig — Defcon 1 and Defcon 3

**PRD:** [`01-multisig-ui.md`](../0-prd/01-multisig-ui.md) — Requirement 15.4 (*"Security Council multisig: 1. Defcon 1 transaction, 2. Defcon 3 transaction"*); Roles section (*"Strata Security Council Signer"*); Requirement 12.2 (no Approved/Canceled state for Security Council).
**Stories:** [`story-map.md`](../3-stories/story-map.md) US-E12 (Create Defcon 1 transaction), US-E13 (Create Defcon 3 transaction).
**Deps:** [`adrs/001-alpen-crate-dependencies.md`](../architecture/adrs/001-alpen-crate-dependencies.md) — pin-update procedure (P0 prerequisite).
**Status:** Designed — pending review.

> **Supersedes stale discovery claims.** [`08-alpen-crate-prd-coverage.md`](../2-discovery/08-alpen-crate-prd-coverage.md) and [`19-asm-bump-impact-assessment.md`](../2-discovery/19-asm-bump-impact-assessment.md) marked Defcon 1/3 as **Blocked — no upstream presence**. That is no longer true: `alpenlabs/asm` implemented the Security Council role and both Defcon actions in commit `3d45351` (*feat(admin): add Security Council and Defcon actions*, PR #81). This spec is written against asm HEAD `71e8287`.

## Objective

Enable the **Strata Security Council** authority end-to-end in the multisig app: a Security Council signer can authenticate, propose a **Defcon 1** or **Defcon 3** emergency transaction, collect quorum signatures, broadcast via the existing commit/reveal pipeline, and track enactment — with UX treatment appropriate for actions that sweep all bridge funds to the safe harbour.

Prerequisite: bump the workspace's six `alpenlabs/asm` git pins past the Defcon commit (P0), which is a **wire-format-breaking** upgrade (see Protocol Recap).

## Scope

### Included

- **P0**: asm pin bump `e0461f8` → Defcon-capable rev, compile fixes, regression verification of all existing flows, and the operational reset procedure (DB / runner / regtest).
- Security Council authentication path (multisig selection, nonce signing, session) — Requirement 7.4 / 8.
- Defcon 1 and Defcon 3 action building, signing, signature collection, and broadcast — reusing the existing commit/reveal pipeline unchanged.
- Per-action (instead of per-authority) lock-period resolution in the orchestrator.
- Defcon enactment detection (safe-harbour activation) in ASM reconciliation.
- Emergency-action signer-safety UX (dedicated section below).
- e2e coverage for the full Defcon flow.

### Not included

- **Security Council Signer update** (membership rotation): authorized by the **Strata Administrator** multisig (`UpdateTxType::StrataSecurityCouncilMultisigUpdate = 15`), not by the council itself — PRD Requirement 15.2.4, separate feature.
- **`SafeHarbourAddressUpdate`** and other new upstream update types: codecs gain explicit "unsupported" arms only (P0), no product flow.
- **Cancel for Defcon 3**: not implemented — see Open Questions (a).
- Bridge/safe-harbour fund handling: protocol concern, enforced on-chain by the ASM.

## Requirements Alignment

- **PRD Requirement 15.4**: Security Council signers can propose Defcon 1 and Defcon 3 transactions.
- **PRD Requirement 12.2** (cited as §5.2.2 in [`cancel-approved-proposal.md`](./cancel-approved-proposal.md)): the Security Council multisig does not produce update types with an "Approved" or "Canceled" state — see State Model for how this reconciles with the uniform backend lifecycle.
- **Orchestrator remains coordination-only**: collects signatures, tracks lifecycle, reports txids; all Defcon validity rules (role authorization, seqno, threshold) are enforced on-chain by SPS-65 / the ASM.
- **Signer safety**: payload-less emergency actions get the strongest confirmation gate in the app (type-to-confirm + verbatim signing-message review).
- **Manual survivability**: signature copy/paste and manual broadcast work for Defcon proposals exactly as for every other proposal type.

## Protocol Recap

Upstream (asm `71e8287`, since commit `3d45351` / PR #81):

- `Role::StrataSecurityCouncil` (`asm/crates/params/src/subprotocols/admin/roles.rs`), canonical name **"Strata Security Council"** (byte-stable: it appears in signing messages). Membership is rotated by `Role::StrataAdministrator`, not by the council — the council cannot lock itself out via self-rotation.
- `UpdateTxType::Defcon1 = 41` and `UpdateTxType::Defcon3 = 43` (`updates.rs`); `authorized_role()` returns `StrataSecurityCouncil` for both. The `40..=49` discriminant band is reserved for the Security Council.
- `Defcon1Update` and `Defcon3Update` are **payload-less SSZ unit structs** wrapped as `MultisigAction::Update(UpdateAction::Defcon1(..))` / `..Defcon3(..)`. The action's identity *is* the signal.
- **Defcon 1** — immediate sweep authorization: signals the bridge to activate its safe harbour immediately. `ConfirmationDepths::get` hardcodes its depth to `0` → bypasses the confirmation queue entirely, **cannot be cancelled**, and has no per-deployment knob.
- **Defcon 3** — delayed sweep authorization: same signal, but timelocked by the configurable `ConfirmationDepths::defcon3` depth → enters the ASM confirmation queue like other updates.
- Enactment mechanism: the admin subprotocol relays `BridgeIncomingMsg::Defcon` to the bridge, which calls `activate_safe_harbour()`. `BridgeV1State` exposes `safe_harbour().is_activated()` — the observable post-condition for reconciliation.
- Signing message (standardized; fixture in upstream `defcon1.rs` test):

  ```
  Strata ASM Administration v1
  Action: Defcon 1
  Authorized By: Strata Security Council
  Sequence: <seqno>
  ```

- Seqno: the Security Council authority exists in the genesis admin state with `last_seqno = 0`; the existing `last_seqno_for_authority` works unchanged once the role is mapped.

### Wire-format break at the pin bump (P0 driver)

At the current pin `e0461f8`, `UpdateAction` has 8 variants. At HEAD, `StrataSecurityCouncilMultisig` is **inserted at position 3** (and `Defcon1`, `Defcon3`, `SafeHarbourAddress` appended). The SSZ union selector for `OperatorSet` and every later variant shifts by one:

- Any `action_hex` persisted **before** the bump decodes to the wrong variant (or garbage) **after** the bump.
- `ActionId = hash(MultisigAction, SeqNo)` values computed pre-bump are not comparable post-bump.

Consequences (mandatory in P0): reset the orchestrator database (or mark all pre-bump proposals terminal), rebuild `strata-asm-runner` from the same asm commit, delete the runner DB (`[database].path` in asm-config.toml, e.g. `/tmp/asm-runner-db`), and reset the regtest datadir so genesis is recreated.

## State Model

**Key decision: the backend lifecycle stays uniform; the UI re-labels, it does not re-model.**

PRD Requirement 12.2's "no Approved/Canceled state" is a **protocol** statement: on-chain, Defcon 1 never sits in a cancellable confirmation window. Off-chain, the coordination phase "quorum collected, broadcast not yet confirmed" genuinely exists for Defcon proposals too — signature collection and broadcast are user steps regardless of action type.

- **Backend — no changes.** Defcon proposals are ordinary `Proposal` rows: `Pending → Approved → Enacted` (+ `Expired`). `action_hex` stays opaque, no DB schema changes, no Security Council special-casing in lifecycle transitions.
- **UI — Security Council display rules:**
  - `Approved` renders as **"Quorum reached — ready to broadcast"**. The word "Approved" is never shown for Security Council proposals, and they are never grouped under any cancellable/"Approved updates" framing.
  - No cancel CTA anywhere (the backend already rejects non-AlpenAdmin/StrataAdmin cancel targets with `400` — this becomes a stated invariant with a test).
  - **Defcon 1**: copy "Enacts immediately once the transaction confirms — cannot be cancelled." No activation-window UI. After reveal confirmation, lock period 0 + the safe-harbour check flip it to `Enacted` on the next reconcile poll, so the on-screen `Approved` phase is transient.
  - **Defcon 3**: activation countdown reusing the existing `activation_height` plumbing, with copy "Timelocked sweep; cancellation is not supported in this application — pending protocol clarification."
- **`Canceled` is structurally unreachable** for Security Council proposals. Invariant test in P1.

## Slices

| Slice | Delivers | PRD coverage | Depends on |
|---|---|---|---|
| **P0 — asm pin bump + regression** | Workspace on a Defcon-capable asm rev; all existing flows verified green; reset procedure documented and executed | — (prerequisite) | — |
| **P1 — Orchestrator Security Council support** | Role mapping, membership/threshold/seqno for SC; per-action lock period; Defcon enactment detection | Req 8.1 (canonical signer set), backend guidelines §3 | P0 |
| **P2 — Tauri action building** | `Action::Defcon1`/`Defcon3`, codec, builder command, decode rendering, broadcast key ordering for SC | Req 15.4 (action construction) | P0 (independent of P1) |
| **P3 — Frontend SC auth + create flow** | SC authentication; Defcon create flow with emergency confirmation gate | Req 7.4, 8, 15.4; US-E12/US-E13 | P2 |
| **P4 — Lifecycle display** | SC status labels, no-cancel invariant in UI, Defcon 3 countdown | Req 12.2, 13, 14 | P1 + P3 |
| **P5 — e2e** | Full propose→quorum→broadcast→enacted flow asserted against a real ASM | acceptance | P1–P4 |

P1 and P2 are independent and can land in either order after P0.

### P0 — asm pin bump + regression

1. Bump the six `rev = "e0461f8…"` pins in root [`Cargo.toml`](../../Cargo.toml) (lines 15–20) to the chosen rev — **all six to the same rev** per ADR-001. Target rev: see Open Questions (d); default HEAD `71e8287`.
2. Fix compile breaks. Exhaustive matches over `UpdateAction`/`UpdateTxType` gain **explicit arms** for every new variant — variants the product does not support return an explicit "unsupported" error, **never `_ =>` catch-alls** (preserves compile-time forcing on the next bump). Expected sites:
   - `orchestrator-be/src/infrastructure/action_codec.rs`
   - `orchestrator-be/src/infrastructure/asm_enactment.rs` (`extract_multisig_config_update` must now consider `StrataSecurityCouncilMultisig` in its wrong-authority arm)
   - `desktop-app/src-tauri/src/infrastructure/action_codec.rs` (`from_strata_action`)
   - e2e-tests fixtures
3. Operational reset (wire-format break): reset orchestrator Postgres (or mark all pre-bump proposals terminal); rebuild `strata-asm-runner` from the same asm commit; delete the runner DB (`/tmp/asm-runner-db` per asm-config.toml); reset the regtest datadir (`~/.bitcoin/asm-runner-regtest`).
4. Regression gate before any Defcon work: `cargo test --workspace`, existing e2e suites (proposal lifecycle, cancel, signer update enacted, vk update enacted), and the webdriver wallet smoke.

### P1 — Orchestrator Security Council support

All in `orchestrator-be`:

- [`src/infrastructure/asm_role_membership.rs`](../../orchestrator-be/src/infrastructure/asm_role_membership.rs):
  - `authority_to_role_impl`: add `Authority::SecurityCouncil => Ok(Role::StrataSecurityCouncil)` (only `PayoutAdmin` remains unsupported). Update the `all_five_authorities_have_explicit_asm_mapping_status` test.
  - `fetch_role_membership`: include `Role::StrataSecurityCouncil` keys.
  - **Replace `lock_period_for_authority(rpc_url, authority)` with `lock_period_for_action(rpc_url, action_hex)`**: decode via `action_codec::decode_multisig_action_hex`, take `UpdateAction::update_tx_type()`, return `admin.confirmation_depth(tx_type).unwrap_or(0)`. Rationale: the Security Council has **two** tx types with different depths (Defcon 1 always 0; Defcon 3 configurable), so the per-authority mapping is wrong by construction. Delete `authority_to_update_tx_type` (this was its only consumer). Sole caller to update: `compute_and_store_activation_height` in `src/application/proposals.rs`. Defcon 1 naturally yields `activation_height = reveal_confirm_block`.
  - Mocks (`mock_membership`, `mock_threshold`, `mock_last_seqno`): add SecurityCouncil values mirroring the StrataAdmin pattern. `mock_lock_period` becomes action-aware: Defcon 1 → 0; Defcon 3 → a small fixed value (e.g. 5); others → 2016.
- [`src/infrastructure/asm_enactment.rs`](../../orchestrator-be/src/infrastructure/asm_enactment.rs): add **explicit** `UpdateAction::Defcon1(_)` / `Defcon3(_)` arms. Without them the catch-all path returns "not enacted" forever — a Defcon proposal would silently never reach `Enacted`. Enacted post-condition:
  - `bridge.safe_harbour().is_activated()` (reuse the existing `decode_bridge_state`) **and** `admin.authority(StrataSecurityCouncil).last_seqno() >= seq_no`;
  - **Defcon 3 additionally**: the update is absent from `admin.queued()` (so a queued-but-not-activated Defcon 3 is not marked Enacted just because a Defcon 1 fired). Residual ambiguity with concurrent Security Council actions is accepted — same risk class already documented in the file header.
- `reconcile_enacted_for_authority` / `reconcile_enacted_for_action` in `src/application/proposals.rs`: **no changes** — authority/status generic.
- No handler changes, no DB schema changes, no auth changes: membership, threshold, seqno, and session flows are authority-generic once the role is mapped.

### P2 — Tauri action building (`desktop-app/src-tauri`)

Reference pattern: SequencerKeyUpdate (commit `2039a62`).

- [`src/domain/action.rs`](../../desktop-app/src-tauri/src/domain/action.rs): add `Action::Defcon1` and `Action::Defcon3` as **unit variants** (mirroring the payload-less upstream structs).
- [`src/infrastructure/action_codec.rs`](../../desktop-app/src-tauri/src/infrastructure/action_codec.rs): `to_strata_action` / `from_strata_action` arms → `MultisigAction::Update(UpdateAction::Defcon1(Defcon1Update))` etc. Tests: round-trip + **golden-hex fixture** (the SSZ encoding of a payload-less action is a deterministic constant — pin it).
- [`src/commands/action_builder.rs`](../../desktop-app/src-tauri/src/commands/action_builder.rs): one command `build_defcon_action_hex` with a validated level field accepting exactly `1 | 3` (single registration point; invalid level → typed error). Returns `BuildActionHexResponse`. Extend `DecodedAction` with `Defcon1`/`Defcon3` variants so `decode_action_hex` renders them on the detail screen. Register in `commands/mod.rs` + the invoke handler list.
- [`src/infrastructure/asm_role_membership.rs`](../../desktop-app/src-tauri/src/infrastructure/asm_role_membership.rs): add `Authority::SecurityCouncil => Ok(Role::StrataSecurityCouncil)` to `authority_to_role` so `ordered_keys_for_authority` (signer-index ordering at broadcast) works for Security Council proposals.
- Signing: **no changes** — `SigningMessage::for_action` renders Defcon messages once the crates are bumped. Required test: fixture asserting the exact 4-line message for a known seqno (mirrors the upstream test).

### P3 — Frontend Security Council auth + create flow (`desktop-app/src`)

Auth path:

- [`types/auth-role.ts`](../../desktop-app/src/types/auth-role.ts): add `AuthRole.StrataSecurityCouncil`.
- [`api/orchestrator-auth.ts`](../../desktop-app/src/api/orchestrator-auth.ts) `authorityFromRole`: add the `'security_council'` arm and **make the switch exhaustive — remove the silent `default: return 'strata_admin'` fallback**. The fallback is a signer-safety hazard: an unmapped role would silently authenticate against the wrong authority. An unknown role must be a thrown error.
- [`lib/authority-label.ts`](../../desktop-app/src/lib/authority-label.ts): label "Strata Security Council".
- Wallet-connect / multisig-select screen: Security Council option (Requirement 7.4).
- `api/ipc-schemas.ts`: extend authority/role enums.

Create flow (`domain/create-proposal/`):

- `model/create-proposal.types.ts`: `ActionType` union + `'defcon1' | 'defcon3'`.
- `model/action-type-config.ts`: entries for both (emergency-flagged copy); `ACTION_TYPES_BY_AUTHORITY.security_council = ['defcon1', 'defcon3']` — **no** signer-update entry (rotation belongs to Strata Admin).
- `model/create-proposal.schema.ts`: confirmation-only schema (no payload fields).
- New `components/defcon-form-fields.tsx`: **no inputs** — warning panel + the verbatim signing message as the reviewable payload + type-to-confirm gate (see Signer-Safety UX).
- `hooks/use-create-proposal.ts` + `api/action-builder.ts`: wire `build_defcon_action_hex`.

### P4 — Lifecycle display

- [`screens/proposal-detail-screen.tsx`](../../desktop-app/src/screens/proposal-detail-screen.tsx) and the proposals dashboard: Security Council status labels per the State Model ("Quorum reached — ready to broadcast" instead of "Approved"); decoded Defcon rendering; authority badge; **no cancel CTA** for Security Council proposals (UI assertion in addition to the backend 400); Defcon 3 activation countdown reusing the existing `activation_height`/countdown components from the cancel spec.

### P5 — e2e

- New `e2e-tests/tests/e2e_defcon_enacted.rs`, modeled on `e2e_signer_update_enacted_light.rs`: propose Defcon 1 → quorum → commit/reveal broadcast → assert `safe_harbour().is_activated()` on bridge state **and** orchestrator status `Enacted`.
- Defcon 3 variant: assert presence in `admin.queued()` before the depth elapses, enactment after.
- Optional webdriver spec `desktop-app/e2e-webdriver/test/specs/proposal-defcon.e2e.js` (runs individually per that package's README).

## Signer-Safety UX

Defcon actions authorize sweeping **all bridge funds** to the safe harbour. They get the strongest gate in the app:

- **Distinct destructive visual treatment** — unmistakably different from every other action form (danger palette, emergency framing).
- **Authority context on every step**: "Strata Security Council" badge through create → review → sign → broadcast.
- **The payload review is the signing message itself.** Since the action carries no payload, the form renders the exact 4-line protocol signing message verbatim — what the user reads is byte-identical to what the hardware wallet displays (Requirement 6.6).
- **Type-to-confirm**: the user must type `DEFCON 1` (or `DEFCON 3`) before the sign CTA enables.
- **Severity copy**: Defcon 1 — "Immediate and irreversible. Enacts as soon as the transaction confirms; it cannot be cancelled." Defcon 3 — "Timelocked: the sweep activates after N blocks. Cancellation is not supported in this application."
- **High-signal errors**: a non-Security-Council session can never reach these forms; a Security Council session sees only Defcon actions.

## Edge Cases

| Scenario | Behavior |
|---|---|
| Signer not on the Security Council canonical set | Existing membership check rejects at proposal creation / auth (Requirement 8.1). |
| Duplicate Defcon proposal (same action + seqno) | Same `ActionId` → existing dedup applies unchanged. |
| Defcon 1 and Defcon 3 proposals coexist | Both activate the safe harbour. Enactment check disambiguates: Defcon 3 is only `Enacted` when activated **and** absent from the queue; `last_seqno >= seq_no` required for both. Residual ambiguity accepted (documented risk class). |
| Cancel attempted against a Security Council proposal | Backend `400` (existing guard); no cancel CTA in UI (P4 assertion). |
| Defcon proposal expires before quorum | Standard 7-day expiry applies (Requirement 13.3) — see Open Questions (c). |
| Broadcast when safe harbour already active | Protocol-level outcome; orchestrator reconciliation marks `Enacted` via the seqno + activation post-condition. UI shows the standard enacted state. |
| Pre-bump proposals in the DB after P0 | Wire-incompatible — DB reset / terminal-marking in the P0 procedure prevents this state from existing. |

## Open Questions (for Alpen)

1. **Defcon 3 cancellability.** Upstream queues Defcon 3 in a confirmation window that SPS-65 cancels can target, but PRD Requirement 12.2 says the Security Council has no Approved/Canceled states. We do **not** implement cancel for Defcon 3; please confirm this is the intended product behavior (and which authority would sign such a cancel if it ever is).
2. **Discovery-doc claim "Security Council actions execute immediately on quorum"** ([`01-conceptual-overview.md`](../2-discovery/01-conceptual-overview.md)) is superseded for Defcon 3, which is now timelocked upstream. Confirm the PRD's intent matches upstream.
3. **Expiry**: should Defcon proposals be exempt from (or have a shorter) standard 7-day pending-expiry window, given their emergency nature?
4. **Pin target**: exact asm rev to pin — current HEAD `71e8287` or a tagged release, per ADR-001.

## Critical Files

| File | Change |
|---|---|
| `Cargo.toml` (root, lines 15–20) | P0 — bump six asm pins to the same rev |
| `orchestrator-be/src/infrastructure/asm_role_membership.rs` | P1 — SC role mapping, membership, mocks; `lock_period_for_action` |
| `orchestrator-be/src/infrastructure/asm_enactment.rs` | P0 compile arms; P1 — explicit Defcon enactment detection |
| `orchestrator-be/src/application/proposals.rs` | P1 — `compute_and_store_activation_height` calls `lock_period_for_action` |
| `desktop-app/src-tauri/src/domain/action.rs` | P2 — `Defcon1`/`Defcon3` unit variants |
| `desktop-app/src-tauri/src/infrastructure/action_codec.rs` | P0 arms; P2 — Defcon codec + golden-hex tests |
| `desktop-app/src-tauri/src/commands/action_builder.rs` | P2 — `build_defcon_action_hex`, `DecodedAction` variants |
| `desktop-app/src-tauri/src/infrastructure/asm_role_membership.rs` | P2 — SC arm in `authority_to_role` |
| `desktop-app/src/types/auth-role.ts`, `api/orchestrator-auth.ts`, `lib/authority-label.ts` | P3 — SC auth; exhaustive `authorityFromRole` |
| `desktop-app/src/domain/create-proposal/` (types, config, schema, form, hook) | P3 — Defcon create flow |
| `desktop-app/src/screens/proposal-detail-screen.tsx` + dashboard | P4 — SC status labels, no-cancel, countdown |
| `e2e-tests/tests/e2e_defcon_enacted.rs` | P5 — new |

## Verification

Per slice, the repo's pre-commit gate plus:

```bash
# Rust (repo root)
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Frontend (desktop-app/)
npm run format:check && npm run lint && npm run build
```

- **P0 done when**: workspace green at the new pin; all pre-existing e2e suites pass; reset procedure executed and documented in the PR.
- **P1 done when**: unit tests cover SC membership/threshold/seqno via mocks, `lock_period_for_action` for Defcon 1 (0) / Defcon 3 (depth) / legacy types, and Defcon enactment arms (including the Defcon 3 queued-not-enacted case).
- **P2 done when**: codec round-trip + golden-hex fixtures pass; signing-message fixture matches the upstream 4-line format exactly.
- **P3/P4 done when**: manual regtest flow — authenticate as a Security Council signer, create Defcon 1, type-to-confirm gate enforced, sign, reach quorum, broadcast, observe "Quorum reached" label (never "Approved"), no cancel CTA, proposal reaches Enacted after safe-harbour activation; Defcon 3 shows the countdown.
- **P5 done when**: `cargo test -p alpen-multisig-e2e-tests` green including `e2e_defcon_enacted`.
