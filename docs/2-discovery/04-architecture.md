# Architecture Overview

This document defines the baseline architecture for the Alpen Multisig application. It serves as the reference for all implementation decisions going forward.

## System Context

Alpen Multisig is a desktop application that enables authorized signers to manage on-chain governance of the Strata bridge and Alpen rollup. The system coordinates signature collection off-chain, constructs Bitcoin transactions embedding governance payloads (SPS-50/51/65), and broadcasts them for the ASM (Administration State Machine) to process deterministically.

```
┌─────────────────────────────────────────────────────────────────┐
│                        Desktop App (Tauri)                      │
│  ┌───────────────────────────┐  ┌────────────────────────────┐  │
│  │   React Frontend (UI)     │  │   Tauri Rust Shell         │  │
│  │   - Wallet connect        │  │   - HWI subprocess mgmt    │  │
│  │   - Auth flow             │  │   - Signing bridge         │  │
│  │   - Proposal management   │  │   - IPC commands           │  │
│  │   - Signature collection  │  │                            │  │
│  │   - Tx broadcast          │  │                            │  │
│  └───────────┬───────────────┘  └────────────┬───────────────┘  │
│              │ HTTP                           │ Tauri IPC        │
└──────────────┼───────────────────────────────┼──────────────────┘
               │                               │
               ▼                               ▼
┌──────────────────────────┐    ┌──────────────────────────────┐
│   Orchestrator Backend   │    │   Hardware Wallet (HWI)      │
│   (Axum + Postgres)      │    │   - Taproot signing          │
│   - Session auth         │    │   - m/86'/0'/73'/0/n         │
│   - Proposal CRUD        │    │   - BIP-137 ECDSA            │
│   - Signature aggregation│    └──────────────────────────────┘
│   - Lifecycle tracking   │
└──────────────────────────┘
               │
               │ (signers broadcast independently)
               ▼
┌──────────────────────────────────────────────────────────────┐
│                      Bitcoin (L1)                             │
│   OP_RETURN (SPS-50) + Witness Envelope (SPS-51)             │
│   ┌────────────────────────────────────────────────────────┐ │
│   │ Strata Node → ASM (Administration State Machine)       │ │
│   │   - Parses admin txs from Bitcoin blocks               │ │
│   │   - Verifies threshold signatures (SPS-65)             │ │
│   │   - Manages queued updates + confirmation depth        │ │
│   │   - Enacts governance changes deterministically        │ │
│   └────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

## Governance Model — Five Authorities

Each authority is an independent multisig with its own signer set, threshold, and sequence number. They are fully isolated — no cross-authority data leakage.

| Authority | Role | Key Actions |
|-----------|------|-------------|
| **Alpen Admin** | Rollup governance | Verification keys, admin signer set updates |
| **Strata Admin** | Bridge governance | Safe harbor, verification keys, signer sets, security council, operators, bridge params |
| **Sequencer Manager** | Sequencer ops | Sequencer key rotation |
| **Security Council** | Emergency response | Defcon 1/3 emergency actions |
| **Payout Admin** | Payout operations | `block_payout` transaction construction and broadcast |

## Component Architecture

### 1. Orchestrator Backend (`orchestator-be`)

**Role:** Off-chain coordination service only. It does NOT enforce protocol validity rules — that is the ASM's job.

**Allowed:** Hygiene checks (malformed input, duplicate signatures, structural consistency).
**Forbidden:** Re-implementing signature threshold verification, sequence number validation, or any canonical SPS-65 logic.

```
orchestator-be/src/
├── main.rs              # Axum app setup, router, middleware stack
├── config.rs            # Env-based configuration (host, port)
├── state.rs             # AppState (config + future DB pool)
├── error.rs             # AppError → HTTP status mapping
├── domain/
│   ├── authority.rs     # Authority enum (5 roles), SignerPubkey, SignerSet
│   ├── session.rs       # Ephemeral session model, AuthChallenge
│   └── proposal.rs      # Proposal, ProposalSignature, QuorumStatus, ActionId, SeqNo
├── handlers/
│   ├── auth.rs          # GET /auth/challenge, POST /auth/session, DELETE /auth/session
│   ├── proposals.rs     # GET/POST /proposals, GET /proposals/:action_id
│   └── signatures.rs    # POST/GET /proposals/:action_id/signatures
└── middleware/
    └── auth.rs          # AuthenticatedSession extractor (Bearer token)
```

**API Surface (`/api/v1`):**

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/health` | Liveness check |
| `GET` | `/auth/challenge` | Generate nonce for signer authentication |
| `POST` | `/auth/session` | Create ephemeral session (attestation signature required) |
| `DELETE` | `/auth/session` | Revoke session |
| `GET` | `/proposals` | List proposals (optional status filter) |
| `POST` | `/proposals` | Create proposal (`seq_no` + `action_payload`) |
| `GET` | `/proposals/:action_id` | Get proposal details + quorum status |
| `POST` | `/proposals/:action_id/signatures` | Submit signature |
| `GET` | `/proposals/:action_id/signatures` | List collected signatures |

**Authentication Model:**

```
Signer (canonical key)                    Backend
       │                                     │
       │  1. GET /auth/challenge             │
       │────────────────────────────────────>│
       │     { nonce, expires_at }           │
       │<────────────────────────────────────│
       │                                     │
       │  2. Sign attestation with HW wallet │
       │     (binds: ephemeral_key +         │
       │      authority + nonce + expiry)    │
       │                                     │
       │  3. POST /auth/session              │
       │     { ephemeral_pubkey, nonce,      │
       │       attestation_signature,        │
       │       signer_pubkey, authority }    │
       │────────────────────────────────────>│
       │     { session_id, expires_at }      │
       │<────────────────────────────────────│
       │                                     │
       │  4. Subsequent requests use         │
       │     Bearer <session_token>          │
       │     signed with ephemeral key       │
       │────────────────────────────────────>│
```

Sessions are nonce + expiry bounded and scoped to exactly one authority.

**Data Identity:**

- `ActionId = hash(MultisigAction, SeqNo)` — deterministic, idempotent
- `SeqNo` is `u64`, allows gaps (not strictly sequential)
- Duplicate `(MultisigAction, SeqNo)` pairs are rejected

**Proposal Lifecycle:**

```
                    ┌──────────┐
                    │ Pending  │ (7-day expiry window)
                    └────┬─────┘
                         │
              ┌──────────┼──────────┐
              │          │          │
              ▼          ▼          ▼
        ┌──────────┐ ┌────────┐ ┌─────────┐
        │ Approved │ │Expired │ │Canceled │
        │(broadcast│ │(timeout│ │(by auth)│
        │ on-chain)│ │pre-    │ │         │
        └────┬─────┘ │quorum) │ └─────────┘
             │        └────────┘
      ┌──────┼──────┐
      │             │
      ▼             ▼
┌──────────┐  ┌──────────┐
│ Enacted  │  │ Canceled │
│(after    │  │(during   │
│confirm.  │  │ waiting  │
│depth)    │  │ period)  │
└──────────┘  └──────────┘
```

### 2. Desktop App (`desktop-app`)

**Tauri Shell** (`src-tauri/`): Rust process managing HWI subprocess, IPC commands, and system-level operations.

**React Frontend** (`src/`): UI layer for all signer interactions.

```
desktop-app/src/
├── main.tsx             # React mount
├── App.tsx              # Root component (currently hello world stub)
├── types/index.ts       # Domain types (Authority, Proposal, Session, QuorumStatus, etc.)
├── api/
│   ├── client.ts        # Typed fetch wrapper → ApiResult<T>
│   ├── auth.ts          # Challenge/session API calls
│   └── proposals.ts     # Proposal/signature API calls
└── hooks/
    ├── useAuth.ts       # Auth state machine (unauthenticated → authenticating → authenticated)
    ├── useWallet.ts     # Wallet connection state (disconnected → connecting → connected)
    └── useProposals.ts  # Proposal loading with status filter
```

**Required Navigation Flow:**

```
Wallet Connect → Address Select → Authority Select → Nonce Sign Auth → Dashboard
```

**Key Frontend Constraints:**
- Never expose private keys in UI state, logs, or storage
- Authority labeling required on every action form
- Quorum progress (`collected / required`) always visible for pending actions
- Support copy/paste signature workflows for manual fallback
- Deterministic payload summary before hardware wallet signing prompt
- Explicit fee inputs for broadcast (0.1 sat/vB increments, max 10,000 sat/vB)

### 3. E2E Tests (`e2e-tests`)

Separate crate (not a workspace member) with its own `rust-toolchain.toml` (nightly). Tests the full admin action flow against real Alpen/Strata crates:

1. Generate signer keys → 2. Build `MultisigAction` → 3. Compute SPS-65 sighash → 4. ECDSA sign (threshold) → 5. Construct Bitcoin tx (SPS-50 OP_RETURN + SPS-51 witness) → 6. Parse back and verify signatures

**Protocol crate dependencies** (pinned):
- `alpenlabs/alpen` @ rev `308211f` — `strata-asm-txs-admin`, `strata-crypto`, `strata-asm-params`, `strata-primitives`, `strata-asm-common`, test utils
- `alpenlabs/strata-common` @ tag `v0.1.0-alpha-rc11` — `strata-l1-txfmt`

## Protocol Integration Points

### SPS-50 — Transaction Header (OP_RETURN)

```
OP_RETURN <magic(4)> <subprotocol_id(1)> <tx_type(1)> <aux(≤74 bytes)>
```

- `subprotocol_id = 0` for administration
- `tx_type` identifies the action (e.g., `10` = StrataAdminMultisigUpdate)

### SPS-51 — Witness Envelope

```
OP_FALSE OP_IF <520-byte chunks of Borsh-serialized SignedPayload> OP_ENDIF
```

- Max payload ~395KB
- Contains: `{ seqno, action, signatures }`

### SPS-65 — Administration Subprotocol

**Sighash computation (domain-separated, replay-protected):**

```
sighash = SHA256( SHA256(tag) || seqno_be_bytes(8) || sighash_payload )
tag = "strata/admin/<type_name>"
```

**ASM verification flow:**
1. `payload.seqno > authority.last_seqno` (replay protection)
2. `payload.seqno ≤ authority.last_seqno + max_seqno_gap` (gap limit, default 10)
3. Verify ECDSA threshold signatures against current signer set
4. Queue update with `activation_height = current_height + confirmation_depth` (~2016 blocks)

## Offline Survivability

The backend is a convenience layer, not a requirement. If the backend is unavailable:

1. Signers can construct proposals manually
2. Collect signatures via out-of-band channels (copy/paste)
3. Build the Bitcoin transaction locally
4. Broadcast directly to the Bitcoin network

The ASM processes Bitcoin blocks regardless of how the transaction was constructed.

## Tech Stack Summary

| Layer | Stack |
|-------|-------|
| Backend | Rust, Axum 0.7, Tokio, Postgres (planned), `serde`, `tracing` |
| Desktop Shell | Tauri 2, Rust |
| Frontend | React 18, TypeScript 5, Vite 5, TailwindCSS 3, react-router-dom 6 |
| Signing | ECDSA (secp256k1), HWI (Taproot, BIP-137), derivation `m/86'/0'/73'/0/n` |
| Protocol | SPS-50/51/65, Borsh serialization, Alpen admin crate |
| E2E Tests | Rust nightly, pinned Alpen/Strata crates |

## Current State

**Implemented:**
- Domain types and API surface definition (backend + frontend)
- Typed API client and hook state machines (frontend stubs)
- E2E test covering full admin action flow (key gen → tx construction → signature verification)
- Protocol documentation and POC findings

**Pending implementation:**
- Backend: persistence layer (Postgres), auth verification against ASM signer set, proposal lifecycle enforcement, session management
- Desktop: HWI integration, wallet connection flow, proposal creation/signing UI, broadcast flow
- Signing layer: consume Alpen admin crate for payload construction, Bitcoin tx building
- Payout flows: manual + automatic `block_payout` construction
