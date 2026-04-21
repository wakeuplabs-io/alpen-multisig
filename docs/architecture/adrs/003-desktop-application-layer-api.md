# ADR-003: Desktop Application Layer API Design

**Status:** Accepted
**Date:** 2026-04-10
**Context:** The desktop app needs an application layer API that serves as the entry point for all business operations (Tauri commands, future CLI, etc.). The API must align with PRD terminology and be simple for consumers while hiding infrastructure complexity.

## PRD Terminology Analysis

The PRDs use distinct terminology at different layers:

| PRD | Term | Meaning |
|-----|------|---------|
| UI PRD | **update** | What the user sees and interacts with ("Pending updates", "Approved updates", "propose new updates") |
| Backend PRD | **proposal** | The coordination-layer concept ("Proposal creation", "Proposal state tracking", "Proposal Semantics") |
| Backend PRD | **action** / `MultisigAction` | The protocol-level payload (the thing being proposed) |
| Backend PRD | `ActionId` | `hash(MultisigAction, SeqNo)` — deterministic, stable identifier |

The backend code sketch names its trait `MultisigBackend` with methods like `create_update_action(action, seq, sig)` and `approve_action(id, sig)`.

### Decision: naming

The application layer uses **proposal** as its primary coordination concept (aligned with the backend PRD's "Proposal Semantics" section). The orchestrator coordinates **proposals**, each of which wraps a `MultisigAction`. The UI layer may present these as "updates" to the user — that mapping is a presentation concern.

## API Design Principles

### 1. Authority is session-scoped, not per-call

The PRD mandates that sessions are "bound to exactly one authority." Therefore, the application layer **never accepts `authority` as a parameter** — it is implicit from the authenticated session. The orchestrator resolves the authority from the bearer token.

### 2. Signing is external to the application layer

The application layer **never receives private keys**. Signing happens externally:
- **Production:** Hardware wallet (HWI subprocess) signs the sighash
- **POC/testing:** Software signer (`signing::sign_sighash`) used in test harness

The application layer receives a `Signature { signer_pubkey, signature_hex }` — the result of an already-completed signing operation.

### 3. The application layer should accept domain types, not serialized bytes

**Target design (production):**

```rust
/// The user proposes an update. The application layer handles everything.
pub async fn create_update_action(
    client: &dyn OrchestratorClient,
    action: &MultisigAction,    // domain type, not hex
    signature: &Signature,
) -> Result<Proposal, ProposalError>
```

The `seq_no` is resolved internally — the orchestrator provides `get_last_seqno()` (per the PRD code sketch), and the application layer uses the next available value. The `MultisigAction` is serialized to hex at the infrastructure boundary (inside the orchestrator client), not by the consumer.

**Current POC-4 compromise:**

```rust
pub async fn create_update_action(
    client: &dyn OrchestratorClient,
    action_hex: &str,           // serialized — orchestrator doesn't support get_last_seqno yet
    seq_no: u64,                // explicit — will be auto-resolved in production
    signature: &Signature,
) -> Result<Proposal, ProposalError>
```

POC-4 accepts `action_hex` and `seq_no` explicitly because:
- The orchestrator doesn't implement `get_last_seqno()` yet (POC-4 uses in-memory repos)
- `MultisigAction` serialization boundary is not yet defined (desktop builds actions locally using Alpen crates, but the orchestrator needs hex bytes over HTTP)

These will be resolved in subsequent slices when the orchestrator gains `get_last_seqno` and we define the serialization boundary more precisely.

### 4. Domain types are owned by the application layer, not the transport layer

The application layer defines its own domain types (`Proposal`, `ProposalSummary`, `Signature`, `ApprovalResult`). These are **not** the orchestrator's DTOs — the mapping between transport DTOs and domain types happens inside the application layer.

This ensures consumers (Tauri commands, CLI) are decoupled from the orchestrator's JSON contract.

## Target API (production)

Aligned with PRD `MultisigBackend`:

```rust
// Propose a new update with first signature
create_update_action(client, action: &MultisigAction, signature) -> Proposal

// Approve an existing proposal
approve_action(client, action_id, signature) -> ApprovalResult

// Query
get_update_action(client, action_id) -> Proposal
list_proposals(client, status?) -> Vec<ProposalSummary>
get_last_seqno(client) -> u64
```

## Evolution Path

| Trigger | Change |
|---------|--------|
| Orchestrator implements `get_last_seqno` | Remove `seq_no` param from `create_update_action` |
| Serialization boundary defined | Accept `MultisigAction` instead of `action_hex` |
| Cancellation flow (Slice 4) | Add `cancel_action(client, action_id, signature)` |
| Payout Admin (Slice 5) | Evaluate if same API works or needs bounded context |

## Consequences

1. **Positive:** API mirrors PRD language — new team members can trace from PRD to code
2. **Positive:** Consumers never deal with private keys, hex serialization, or session details
3. **Positive:** Transport DTO changes don't ripple to consumers
4. **Trade-off:** POC-4 still exposes `action_hex` and `seq_no` as params — accepted as temporary
5. **Risk:** Domain types and transport DTOs may drift — mitigated by explicit mapping functions in the application layer
