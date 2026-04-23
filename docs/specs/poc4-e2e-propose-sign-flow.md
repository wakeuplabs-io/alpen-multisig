# Spec: POC-4 E2E — Propose → Sign → Verify Flow

## Objective

Integration test that exercises the real desktop application layer (`proposals.rs`) making real HTTP calls to a real orchestrator server. Validates the complete happy path: propose → get → approve → get → verify signatures.

## Scope

**Included:**
- Desktop `lib.rs` exposing `proposals` module (the application entry point)
- Align desktop DTOs to orchestrator's actual JSON responses
- Simplify desktop: no auth, correct URLs, `approve_action` returns `Proposal`
- Start orchestrator as subprocess
- E2E test in `e2e-tests/` using real `proposals::create_update_action`, `approve_action`, etc.
- Real signing

**NOT included:**
- Modifying orchestrator as lib (runs as subprocess)
- list_proposals (not needed for happy path)
- Quorum detection / threshold
- Auth / sessions
- UI / Tauri commands

## Technical Design

### Desktop `lib.rs`

Exposes only the application entry point:

```rust
pub mod application {
    pub mod proposals;
}
```

`orchestrator_client` and `signing` stay internal.

### Desktop DTO alignment (contract mismatches)

| # | Mismatch | Fix |
|---|----------|-----|
| 1 | `CreateProposalRequest` missing `authority` | Add `authority: String` field |
| 2 | Orchestrator wraps response in `CreateProposalResponse` | Orchestrator returns `Proposal` directly |
| 3 | Desktop has multiple response types | Unify to single `Proposal` type |
| 4 | URL `/proposals/:id/signatures` | Change to `/proposals/:id/approve` |
| 5 | `HttpOrchestratorClient` requires bearer auth | Remove auth, just `base_url + reqwest::Client` |

### Orchestrator change (minimal)

`create_proposal` handler returns `(StatusCode::CREATED, Json<Proposal>)` directly.

### E2E test

Lives in `e2e-tests/tests/e2e_propose_sign.rs`. Depends on `desktop-app` lib.

```
1. cargo build -p orchestrator-be
2. Start server subprocess on random port, wait for health check
3. Create HttpOrchestratorClient(base_url)
4. Signer A: generate keypair, build MultisigAction, compute sighash, sign
5. proposals::create_update_action(client, action_hex, seq_no, sig_a) → Proposal (1 sig)
6. proposals::get_update_action(client, action_id) → verify persisted
7. Signer B: generate keypair, compute sighash, sign
8. proposals::approve_action(client, action_id, sig_b) → Proposal (2 sigs)
9. proposals::get_update_action(client, action_id) → verify 2 sigs
10. verify_threshold(pubkeys, threshold=2, sigs, sighash) → valid
11. Kill server
```

### Production code vs. test helpers

**Production:**
- `desktop-app/src-tauri/src/lib.rs` — exposes `proposals`
- Aligned DTOs, simplified trait/client
- Simplified orchestrator `create_proposal` response

**Test helpers (in test file):**
- `TestServer` — manages subprocess (start, health poll, kill on drop)
- Keypair generation, action building, signing

## Test Cases

1. **Happy path: create → get → approve → get → verify_threshold** — real desktop `proposals.rs`, real HTTP, real orchestrator subprocess, real cryptographic signing. Single test covering the full propose → sign → verify flow.

## Module structure

- **`desktop-app/src-tauri/src/lib.rs`** (new) — Exposes `proposals` module.
- **`e2e-tests/tests/e2e_propose_sign.rs`** (new) — E2E integration test.
- Modified: desktop orchestrator_client.rs, desktop proposals.rs, orchestrator handler.
