# Product Owner / Requirements / Multisig UX Safety — Adversarial Assessment

**Assessment date:** 2026-05-14  
**Mode:** Read-only adversarial audit  
**Scope:** User stories, Definition of Ready, multisig UX safety (authority context, confirmation, fallback), alignment of `docs/3-stories/` with PRDs and feature specs.

---

## 1. Scope & threat model

**What we try to break**

1. **Story completeness** — Stories that read “done” but omit negative paths, concurrent state changes, or offline/backend-down behavior.
2. **DoR vacuum** — Work entering design or implementation without an explicit, checkable Definition of Ready (JTBD, AC shape, failure modes, dependencies).
3. **Signer safety** — Payload/authority confusion, weak confirmation, or UI labels that do not match the signer’s mental model.
4. **PRD ↔ story drift** — PRD promises (e.g., manual fallback when backend is down) deferred in the map without acceptance criteria that close the gap.

**Out of scope:** Implementation quality beyond what is visible in story/spec text and a few high-signal UI evidence paths.

---

## 2. Top findings (ranked)

### Blocking / critical

**1. Offline / backend-unavailable path is named in the map but not closed with testable AC**

- **Risk:** Signers believe they cannot act when the orchestrator is down, or teams ship without a specified export/aggregate/broadcast story.
- **Evidence:**
  - `docs/3-stories/story-map.md` — US-H5 “Compose a transaction manually when the backend is unavailable.”
  - `docs/0-prd/02-multisig-backend.md` §2 — When backend unavailable, signers MUST still construct, aggregate, and broadcast without it.
  - No single spec in `docs/specs/` was found that fully specifies desktop “offline/manual” UX, error surfaces, and data export for that path (assessment: gap vs PRD).
- **Adversarial take:** US-H5 is a liability until Given-When-Then AC exists and is traceable to UI + manual steps.

**2. Authority context is easy to misread; at least one screen shows a wrong authority label**

- **Risk:** Multi-authority signers approve or broadcast under the wrong governance role.
- **Evidence:**

```16:17:desktop-app/src/screens/broadcast-proposal-screen.tsx
	const authorityLabel =
		selectedRole === AuthRole.StrataAdministrator ? 'Alpen Administrator' : 'Alpen Sequencer Manager'
```

  - Strata Administrator is labeled “Alpen Administrator” — inconsistent with the intended authority naming elsewhere.
- **Adversarial take:** This is not a polish issue; it directly attacks “explicit confirmation” and “authority context” conventions in `.cursor/rules/general.mdc`.

### High

**3. No Definition of Ready is embedded in the story layer**

- **Risk:** Stories go to spec/implementation without enforced gates (edge cases, dependency on Alpen crates, hardware-wallet confirmation semantics).
- **Evidence:**
  - `docs/3-stories/README.md` — Describes story mapping principles and “what comes next”; **no DoR checklist or mandatory AC format**.
  - `docs/3-stories/story-map.md` — Rich backlog; no standing “Definition of Ready” section.
- **Adversarial take:** Process debt becomes safety debt when signer flows skip failure-mode AC.

**4. Concurrent lifecycle events (cancel / enact while signing) need explicit AC everywhere signing exists**

- **Risk:** Signer completes HW wallet flow after the proposal is no longer signable; ambiguous recovery UX.
- **Evidence:**
  - `docs/specs/proposal-signing-and-dashboard-status-alignment.md` — Describes blocking when not `pending`; adversarial question is whether **every** story that touches signing repeats this and is tested.
  - Story map entries for approve/sign flows (e.g., US-F*, US-I*) should be audited for matching AC; this assessment treats uneven coverage as a requirements risk.

### Medium

**5. “Human-readable” / hardware-wallet parity is still asserted more than specified**

- **Risk:** AC says “signer sees meaningful representation” without binding UI fields to what the device can display (see discovery on Trezor constraints).
- **Evidence:**
  - `docs/2-discovery/16-poc5-trezor-findings.md` — Device/firmware limits affect what signers can verify.
  - Gap between PRD “signer safety” and a single normative “signing confirmation” spec (naming varies; no `*signer-safety*` doc in tree).

### Low

**6. Payout-admin slice dependencies are called out in the map but remain architecturally fragile**

- **Risk:** Stories advance while Bitcoin-native payout assumptions are still discovery-heavy.
- **Evidence:** `docs/3-stories/story-map.md` — Dependencies and risks section (Payout Admin / non-SPS-65 paths).

---

## 3. Attack narratives (3–6)

### N1: Backend outage during urgent rotation

A signer must push a signer-set change while the orchestrator returns 503. US-H5 exists on the map but AC/spec do not tell them what to export, in what format, or how to avoid duplicating proposals after partial failure. They stall or reuse unsafe ad-hoc tooling.

### N2: Wrong authority on broadcast

A Strata Admin signer reaches the broadcast screen; the header reads “Alpen Administrator” due to the swapped label. They believe they are operating under Alpen Admin governance and notify the wrong committee.

### N3: Story shipped without “proposal canceled mid-sign”

Implementation follows a story missing concurrent-state AC. The HW wallet flow completes; the app then errors with a generic message. The signer does not know whether their signature was persisted or replayable.

### N4: DoR bypass for a blocked Alpen update type

A story is scheduled in a slice but upstream `strata_asm_*` does not expose the action variant. Without DoR item “Alpen crate capability verified,” engineers burn time or ship a UI that fails at signing time.

### N5: Expiry while dashboard is open

PRD defines expiry; without AC for polling or user-visible transition, a signer keeps working on a ghost “pending” card until the server rejects the action.

### N6: Manual fallback folklore

Operators assume “manual fallback” is implemented because the PRD requires it; the story map defers US-H5. Incident response has no runbook step that matches product reality.

---

## 4. Evidence index (paths)

| Kind | Path |
|------|------|
| PRD — backend non-SPOF / fallback | `docs/0-prd/02-multisig-backend.md` |
| PRD — UI / signer-facing | `docs/0-prd/01-multisig-ui.md` |
| Story map + US-H5 | `docs/3-stories/story-map.md` |
| Story folder README (no DoR) | `docs/3-stories/README.md` |
| Non-functional extraction | `docs/3-stories/non-functional-items.md` |
| Signing + dashboard spec | `docs/specs/proposal-signing-and-dashboard-status-alignment.md` |
| Trezor / HW constraints | `docs/2-discovery/16-poc5-trezor-findings.md`, `docs/specs/poc5-trezor-hw-wallet-integration.md` |
| Authority label bug | `desktop-app/src/screens/broadcast-proposal-screen.tsx` |
| Dashboard authority reference | `desktop-app/src/screens/proposals-dashboard-screen.tsx` |
| Agent conventions | `AGENTS.md`, `.cursor/rules/general.mdc` |

---

## 5. Smallest fixes vs. largest bets

**Smallest (hours–1 day, requirements/doc)**

- Fix the authority label strings on the broadcast screen and add a cross-screen consistency AC in the relevant stories.
- Add a **DoR subsection** to `docs/3-stories/README.md` or the top of `story-map.md` (8–10 checkboxes: JTBD, GWT AC, authority, failure modes, deps, NFR, test plan).
- Expand **US-H5** with explicit Given-When-Then rows: unreachable orchestrator, export payload, resume when backend returns.

**Largest (weeks)**

- Author an end-to-end **offline / manual coordination** spec (possibly split: export format, signature aggregation, broadcast handoff) and tie it to Slice ordering.
- Add a **signer confirmation model** doc that binds UI blocks + minimum device-display semantics per action family.
- Journey / emotional-arc supplement to the story map for multi-signer collaboration (who waits on whom; what “done” means).

---

## 6. What would change my mind

- A **merged spec** for US-H5 with AC IDs and links to implemented screens/tests would downgrade finding 1 from blocking to medium.
- Evidence that **every** sign/broadcast story includes concurrent-state and authority-display AC would soften finding 3–4.
- A recorded **stakeholder decision** to defer manual fallback past a named milestone, explicitly accepting PRD §2 risk, would change the prioritization narrative (not the safety tradeoff).
