# Security Council — PRD compliance matrix

**PRD source:** [`06-prd-hardware-signer-and-block-payouts-update.md`](../0-prd/06-prd-hardware-signer-and-block-payouts-update.md) (current snapshot)
**Master plan:** [`security-council.md`](./security-council.md)
**Last updated:** 2026-09-22 (Stage 6 audit, against `develop` after #570–#572)

This matrix is the **single place** that records PASS / PARTIAL / FAIL for the PRD
requirements the Security Council feature touches: the council's two actions, the Strata
Administrator's two council-related actions, and the general §5 lifecycle requirements as they
apply to those four. The per-slice contracts say what each slice promised; this says whether the
PRD is met. Every row cites the code that answers it, the caller that reaches it, and the test that
pins it — a row without a `file:line` is a row nobody checked.

## Status legend

| Status | Meaning |
|--------|---------|
| **PASS** | Met for the four Security Council actions (see Notes). |
| **PARTIAL** | Met in part; what is missing is named in Notes. |
| **FAIL** | Not met. |

Rows marked *general* are §5 requirements this feature inherits from the proposal lifecycle every
authority shares; their status is the same for every authority, and they are listed here because
the council actions depend on them.

## Matrix

| # | PRD | Requirement | Status | Evidence (code · reached from · test) | Notes |
|---|-----|-------------|--------|----------------------------------------|-------|
| 1 | §3.1.4 | Security Council multisig usable exclusively by its signers | **PASS** | Membership read `orchestrator-be/src/infrastructure/asm_role_membership.rs:11`, role `:229` · `orchestrator-be/src/handlers/auth.rs:129` · mapping test `payout_admin_is_the_only_unmapped_authority` (same file) | Membership is the live on-chain signer set; auth tests are generic, none is council-specific. |
| 2 | §3.1.4, §5.5 | Only the authorizing role can create each action (council: Defcon 1/3; Strata Admin: council rotation, safe harbor) | **PASS** | `require_authorized_for_action` `asm_role_membership.rs:196` (upstream `authorized_role()`) · `handlers/proposals.rs:105` · four `*_is_authorized_for_*_and_refused_for_*` tests in `asm_role_membership.rs` | The UI menu (`desktop-app/src/domain/create-proposal/model/action-type-config.ts:76`) mirrors it; the backend gate is the one that decides. |
| 3 | §5.5 | Security Council: Defcon 1 transaction | **PASS** | Builder `desktop-app/src-tauri/src/commands/action_builder.rs:206`; enactment `orchestrator-be/src/infrastructure/asm_enactment.rs:108`, predicate `:239` · reconcile in `application/proposals.rs` · `defcon1_*` tests in `asm_enactment.rs`; WebDriver `desktop-app/e2e-webdriver/test/specs/defcon-1-create.e2e.js:20`; activation on chain in `e2e-tests/tests/e2e_safe_harbor_address.rs` and `e2e_council_rotation.rs` | Type-to-confirm and four-line message: contract AC 4–5. |
| 4 | §5.5 | Security Council: Defcon 3 transaction | **PASS** | Builder `action_builder.rs:216`; enactment `asm_enactment.rs:127`, predicate `:255`; live depth `lock_period_for_action` in `asm_role_membership.rs` · reconcile · `defcon3_enacted_needs_every_term`; `e2e-tests/tests/e2e_defcon_probe.rs:47` (activation exactly at depth, not one block before) | Delay is always the live `confirmation_depths.defcon3`, never a constant. |
| 5 | §5.5 | Strata Administrator: Security Council Signer update | **PASS** | Builder `action_builder.rs:123` (role `security_council`); target resolution `asm_enactment.rs:335`/`:351` · reconcile · `e2e-tests/tests/e2e_council_rotation.rs:100` and `:115` | Enactment reads the council's config and the administrator's seqno; the council never sees the proposal that rotates it (contract AC 10). |
| 6 | §5.5 | Strata Administrator: Safe Harbor address update | **PASS** | Builder `action_builder.rs:254`, P2TR/network checks `desktop-app/src-tauri/src/domain/action.rs:152`/`:164`; enactment `asm_enactment.rs:150`, predicate `:286` · reconcile · `e2e-tests/tests/e2e_safe_harbor_address.rs:101`, `:111`, `:121` | A rotation submitted while the harbor is active is accepted on chain and changes nothing; it resolves as `Superseded`, never `Enacted`. |
| 7 | §5.2.2 | Defcon 1 has no Approved / Canceled state | **PASS** | Display `desktop-app/src/lib/proposal-status.ts:75` (*Quorum reached*, never *Approved*); cancel refused by depth `orchestrator-be/src/application/proposals.rs:795`; `is_cancelable` false at depth 0 `asm_role_membership.rs:150` · DTO `handlers/proposals.rs:86` · `test_create_cancel_proposal_rejects_zero_depth_action`; `proposal-display-status.test.ts` | The gate is the action's depth, not its authority. |
| 8 | §5.2, §5.2.1 | Defcon 3 is Approved and cancellable, by the council itself | **PASS** | Cancel creation `application/proposals.rs:764`, authority taken from the target `:814`; affordance `desktop-app/src/domain/proposal-detail/model/derive-proposal-actions.ts:34`/`:71` · `handlers/proposals.rs` cancel endpoint · `e2e_defcon_probe.rs:60` (a cancelled Defcon 3 never activates the harbor), `reconcile_*cancel*` tests | See row 15 for the ASM-down caveat. |
| 9 | §5.2, §5.2.1 | Council rotation and safe harbor update are Approved and cancellable by the Strata Administrator | **PASS** | Same cancel path · `test_strata_admin_rotations_are_cancelled_only_by_the_strata_administrator` (`application/proposals.rs`); `e2e_council_rotation.rs:115`, `e2e_safe_harbor_address.rs:111` | The council cannot cancel a change to who sits on it or where the sweep lands. |
| 10 | §5.2 | Approved updates show their cancellation-signature count | **PASS** *(general)* | `desktop-app/src/domain/proposals-dashboard/components/proposals-dashboard.tsx:563` | The dashboard groups them under *In progress → Quorum reached*, not a group named *Approved* — see row 16. |
| 11 | §5.2.1.1, §5.3.2.1 | Copy cancellation / approval signatures | **PASS** *(general)* | *Copy bundle* `desktop-app/src/domain/proposal-detail/components/proposal-detail.tsx:322`, reachable for a cancel from *View cancel* (#561) | |
| 12 | §5.2.1.2, §5.3.2.2 | Broadcast via the app's RPC **or** by copying the raw transaction | **PARTIAL** *(general)* | App RPC: commit/reveal pipeline. Raw tx: `desktop-app/src/domain/broadcast-proposal/components/send-manually-panel.tsx:15` renders only when a broadcast has failed with `recovery === 'manual-broadcast'` | The raw-tx path is a recovery, not a choice offered up front. |
| 13 | §5.3 | Pending updates show time left before expiry | **FAIL** *(general)* | Desktop `desktop-app/src-tauri/src/config/mod.rs:3` hardcodes `PROPOSAL_EXPIRY_DAYS = 1`; orchestrator default is 7 (`orchestrator-be/src/config.rs:76`) | The countdown a signer sees is computed from 1 day while the backend expires at 7, and the 24 h warning fires from creation. Tracked in #551. |
| 14 | §5.3.3 | A pending update expires after 7 days | **PARTIAL** *(general)* | `expire_if_overdue` `application/proposals.rs:291`, on read from `handlers/proposals.rs:162`/`:211`; default 7 days `config.rs:76` | Met by default, but `expire_if_overdue` has no unit test, and a deployment can set `PROPOSAL_EXPIRY_DAYS` to anything. |
| 15 | §5.2.1 | Cancel is always offered while an update is cancellable | **PARTIAL** | `ConfirmationDepthResolver::Unavailable` answers `None` (`asm_role_membership.rs:143`), which the DTO reports as `is_cancelable: false` | While the ASM RPC is down the Cancel CTA disappears from every surface with no "unknown" state. The on-chain window is unaffected; the signer is not told. Recorded debt in the V2 build plan §6. |
| 16 | §5.2 | "Approved" means quorum **and** confirmed on chain | **PARTIAL** *(general, documented deviation)* | `proposal-status.ts:75` shows *Approved* once quorum is reached, before the broadcast; after the reveal confirms it shows *Awaiting enactment* | The application's *Approved* is the backend status (quorum), one step earlier than the PRD's. It shows in the cancel affordance too: the dashboard offers Cancel only once the update awaits enactment (`proposals-dashboard.tsx:637`), but the detail screen offers it from quorum (`desktop-app/src/screens/proposal-detail-screen.tsx:210-213`), before the update is on chain — a cancel broadcast then would find nothing queued. |
| 17 | §3.2.4 | The signer can read on the device what they sign | **PASS** | `render_signing_message` `desktop-app/src-tauri/src/infrastructure/signing.rs:138`, shown on the create and sign screens · message-shape tests in `commands/action_builder.rs` | Defcon messages are the four header lines with no details block; a safe harbor update shows the BOSD descriptor, not the address. Device behaviour per model: `specs/admin-wallet-prd-compliance.md` §3.2.4. |

## Actions and roles

The scope of each role, the payload each action carries, and when each action executes and can be
cancelled — the definition the feature was blocked on before upstream shipped it — as implemented:

| Action | Authorizing role | Payload | Execution | Cancellation |
|---|---|---|---|---|
| Defcon 1 (tx 41) | Security Council | none | Immediate: activates the bridge safe harbor in the reveal block | Never — depth 0 |
| Defcon 3 (tx 43) | Security Council | none | After the live `confirmation_depths.defcon3` | By the Security Council, inside the window |
| Security Council signer update (tx 15) | Strata Administrator | `ThresholdConfigUpdate` of the council | After the live depth | By the Strata Administrator |
| Safe Harbor address update (tx 14) | Strata Administrator | P2TR BOSD descriptor | After the live depth; accepted and discarded while the harbor is active | By the Strata Administrator |

Sources: master plan [§3](./security-council.md#3-action-inventory) (inventory),
[§2.2](./security-council.md#22-what-happens-when-a-defcon-fires) and
[§3.2](./security-council.md#32-observable-post-conditions) (execution),
[§5.1](./security-council.md#51-defcon-3-is-cancelable--resolved-the-prd-was-corrected) (cancellation).

## Open items

None of these is specific to the Security Council; each is a general §5 behaviour the feature
inherits. They are recorded here, not fixed by Stage 6.

| Row | Item | Tracked |
|---|---|---|
| 13 | Expiry countdown computed from a 1-day desktop constant | #551 |
| 14 | `expire_if_overdue` untested | — |
| 12 | Raw-tx copy offered only after a failed broadcast | — |
| 15 | Cancel affordance disappears silently while the ASM is down | V2 build plan §6 |
| 16 | *Approved* shown at quorum, before on-chain confirmation; the detail screen offers Cancel from that point | — |
