# Plan: Basic Flow — Propose and Sign

> **Post-discovery note (2026-04-17).** This document is the **plan** as written at the start of POC-4. The plan was executed: POC-4 closed successfully, Hardware Wallet Integration (Slice 3) was delivered via POC-5, and `application.rs` evolved from a single file into the [`desktop-app/src-tauri/src/application/`](../../desktop-app/src-tauri/src/application/) and [`orchestrator-be/src/application/`](../../orchestrator-be/src/application/) module directories. See the updated slice statuses in §3 below and the consolidated findings in [`docs/specs/poc4-e2e-propose-sign-flow.md`](../specs/poc4-e2e-propose-sign-flow.md).

## Context — Discovery Phase Closure

This plan defines the implementation strategy for **POC-4: Mini Coordination Flow**, the final proof-of-concept in the discovery phase. It validates that the building blocks proven in POC 1–3 connect into a working end-to-end flow.

| POC | What it proved | Findings | Status |
|-----|----------------|----------|--------|
| POC-1 | Admin subprotocol topology, ASM flow, e2e tx construction + verification | [03-poc1-findings](./03-poc1-findings.md) | Done |
| POC-2 | Tauri + React + IPC architecture, session token isolation in Rust | [04-poc2-findings](./04-poc2-findings.md) | Done |
| POC-3 | Signing library — `compute_sighash`, `sign_sighash`, `verify_threshold` using Alpen crates | [05-poc3-findings](./05-poc3-findings.md) | Done |
| **POC-4** | **Mini coordination flow — propose → sign → quorum across desktop + orchestrator** | **This plan** | **This plan** |

> **Discovery context:** The original POC plan is documented in [02-discovery.md](./02-discovery.md). POC scope evolved during execution — POC-2 shifted from wallet UI to IPC architecture validation, POC-3 from signing UI to signing library, and POC-4 from "mini backend" to full coordination flow. See also [01-conceptual-overview.md](./01-conceptual-overview.md) for protocol background.

**Why POC-4 closes the discovery phase:**
- POC-1 proved we can build and verify admin transactions against Alpen crates ([findings](./03-poc1-findings.md))
- POC-2 proved the desktop architecture (IPC, token isolation) ([findings](./04-poc2-findings.md))
- POC-3 proved we can sign and verify in the desktop app (`signing.rs` — production-ready with 10 tests)
- POC-4 proves these pieces **coordinate through the orchestrator** — the last unknown before production implementation

After POC-4, the architecture is validated and the foundation is ready for feature slices.

## Approach

- Iterative and incremental — each step is independently committable and testable
- Architecture evolves organically — abstractions (traits, services) appear when testing demands them, not before
- Application layer is the focus — no UI, no Tauri commands, no framework concerns
- Auth is mocked — hardcoded signer list per authority
- External dependencies are mocked at each boundary

## Slices — High Level

| Slice | Description | Status |
|-------|-------------|--------|
| **Slice 1 (POC-4)** | Basic flow: propose → sign → quorum (detailed below) | **Done** — see [`docs/specs/poc4-e2e-propose-sign-flow.md`](../specs/poc4-e2e-propose-sign-flow.md) |
| Slice 2 | Bitcoin tx construction (SPS-50/51 envelope) + broadcast | Not started — Phase 3 |
| Slice 3 | Hardware wallet integration (HWI subprocess) | **Done** (Trezor via `trezor-client`, Rust-native) — see [`16-poc5-trezor-findings.md`](./16-poc5-trezor-findings.md) |
| Slice 4 | Cancellations + expiry + past states | Not started — Phase 3 |
| Slice 5 | Payout Administrator flow (`block_payout`) | Not started — Phase 3 |
| Slice 6 | Real auth (ephemeral session keys, nonce signing) | Not started |
| Slice 7 | Postgres persistence | Not started |

Each slice is a horizontal cut — end-to-end functionality that can be demonstrated and tested.

---

## Slice 1 / POC-4 — Propose and Sign (Detailed)

### Functional Flow

```
1. Signer A creates proposal     → desktop builds MultisigAction, computes sighash,
                                    signs, sends to orchestrator
2. Orchestrator persists          → stores proposal + first signature
3. Signer B lists proposals       → desktop fetches pending proposals from orchestrator
4. Signer B signs                 → desktop computes sighash, signs, sends to orchestrator
5. Orchestrator detects quorum    → threshold reached (e.g., 2-of-3)
```

**What is mocked/skipped:**
- Auth — hardcoded signer list, no nonce signing, no ephemeral keys
- Persistence — in-memory repositories (no Postgres)
- Hardware wallet — software keys (secp256k1 keypairs)
- Bitcoin broadcast — no SPS-50/51 tx construction
- UI — no React, no Tauri commands
- Cancellations, expiry, past states

### Step 1: Desktop App — Application Layer

**Goal:** The desktop app can create a proposal and sign it, tested against a mocked orchestrator.

**What gets built:**
- The `application` module gains business logic functions: create proposal (orchestrating `signing.rs` + backend call), sign existing proposal, list proposals <br/>*(post-implementation: this is now the [`desktop-app/src-tauri/src/application/`](../../desktop-app/src-tauri/src/application/) module directory.)*
- A backend client abstraction (trait) is introduced — the test uses a mock implementation, the real implementation (reqwest) is adapted behind it
- Application layer is tested in isolation: real signing (`signing.rs` already works) + mocked backend

**What is NOT touched:** Tauri, commands, state, frontend

**Architecture evolution:** First real abstraction — the backend client trait. The `application` module starts having testable orchestration logic.

### Step 2: Orchestrator — Application Layer

**Goal:** The orchestrator can receive proposals, accumulate signatures, and detect quorum, tested against mocked repositories.

**What gets built:**
- The `application` module implements real logic (replaces `todo!()` stubs): create proposal, receive signature, list proposals, detect quorum
- A proposal repository abstraction (trait) is introduced — in-memory implementation for tests
- A signer set provider abstraction (trait) is introduced — static/hardcoded implementation for tests
- Application layer is tested in isolation: in-memory repos + static signer set

**Key business rules tested:**
- `ActionId = hash(MultisigAction, SeqNo)` — deterministic, idempotent
- Duplicate `(action, seq_no)` rejected
- Duplicate signer signature rejected
- Non-signer signature rejected
- Quorum detection (collected >= threshold)
- Authority isolation (signer of authority A cannot see proposals of authority B)

**Architecture evolution:** Repository and signer set provider abstractions emerge. The `application` module grows with real coordination logic.

### Step 3: Orchestrator — HTTP Handlers

**Goal:** Handlers connect HTTP to the application layer. Real HTTP requests can be made against the orchestrator.

**What gets built:**
- Handlers stop being `todo!()` stubs and delegate to the application layer
- Auth middleware is mocked (injects a fixed session with authority + signer)
- HTTP integration tests: request → handler → application → in-memory repo → response

**Architecture evolution:** HTTP layer is validated as thin (per ADR-002). Handler/application separation works end-to-end within the orchestrator.

### Step 4: Desktop ↔ Orchestrator — Integration

**Goal:** The desktop app uses the real HTTP client against the real orchestrator (with in-memory repos).

**What gets built:**
- The real backend client implementation (reqwest) points to the orchestrator running in test
- Integration test: desktop creates proposal → orchestrator persists → desktop lists → desktop signs → orchestrator detects quorum
- Contract mismatches between mock assumptions and real orchestrator responses are resolved

**Architecture evolution:** Mock from step 1 and real implementation converge. Abstractions are validated as compatible.

**Note:** This step may require backward adjustments if the mock diverged from the real orchestrator — this is expected and healthy.

### Step 5: E2E Test — Horizontal

**Goal:** A single test exercises the complete slice flow: propose → sign → quorum.

**What gets built:**
- Test in `e2e-tests/` that starts the orchestrator and runs the full flow
- Two simulated signers (software keys), one authority, threshold 2-of-3
- Signer A: creates proposal with MultisigAction + first signature
- Signer B: lists proposals, signs
- Assert: quorum reached, proposal status updated

**What this validates:**
- The entire slice works end-to-end
- Abstractions did not leak incorrect details
- SPS-65 sighash computation is consistent between both signers (deterministic)
- The orchestrator correctly tracks state across multiple requests

## Architecture Evolution Summary

| Step | Abstraction introduced | Where |
|------|----------------------|-------|
| 1 | Backend client trait | desktop `application/` module |
| 2 | Proposal repository trait, signer set provider trait | orchestrator `application/` module |
| 3 | — (HTTP wiring only) | orchestrator `handlers/` |
| 4 | Real impl of backend client | desktop `application/` module |
| 5 | — (validation) | `e2e-tests/` |

> Post-implementation: both `application` layers are now module directories (`desktop-app/src-tauri/src/application/` and `orchestrator-be/src/application/`), not single files. The original plan's use of `application.rs` reflects the single-file scaffold at the start of POC-4.

Each abstraction appears when testing demands it. Each step is independently committable. The architecture grows from concrete to abstract as complexity requires.

## Discovery Phase Closure Criteria

POC-4 is complete when:
1. Desktop application layer can orchestrate signing + backend communication (tested with mocks)
2. Orchestrator application layer can manage proposals + signatures + quorum (tested with in-memory repos)
3. Both layers integrate correctly via HTTP (tested with real client + real server)
4. An E2E test demonstrates the full propose → sign → quorum flow
5. Architecture abstractions are validated and ready for production implementations

After POC-4, the discovery phase concludes with:
- **Proven:** Protocol integration (POC-1), desktop architecture (POC-2), signing (POC-3), coordination flow (POC-4)
- **Ready for production:** Clean application layers with tested abstractions
- **Clear path forward:** Subsequent slices add capabilities (Postgres, HWI, auth, broadcast) behind existing interfaces

## What Comes Next

| Slice | Builds on | Adds |
|-------|-----------|------|
| Slice 2 | POC-4 signing flow | SPS-50/51 Bitcoin tx construction + broadcast |
| Slice 3 | POC-2 Tauri architecture | HWI subprocess, real hardware wallet signing |
| Slice 4 | POC-4 proposal lifecycle | Cancellations, expiry, past states |
| Slice 5 | POC-4 orchestrator | Payout Administrator `block_payout` flow |
| Slice 6 | POC-4 auth mock | Real ephemeral session keys, nonce signing |
| Slice 7 | POC-4 in-memory repos | Postgres persistence behind existing traits |
