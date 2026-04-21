# Spec: Domain model for StrataAdmin + MultisigUpdate (POC-4)

## Objective

Introduce a minimal, client-owned domain model for the single action type exercised by
[poc4-e2e-propose-sign-flow.md](./poc4-e2e-propose-sign-flow.md): a `StrataAdmin` authority
proposing a `MultisigUpdate`. Today the application layer and tests talk in raw
`action_hex: String` + `authority: &str`, importing `strata_asm_params`,
`strata_asm_txs_admin` and `strata_crypto` directly from production and test code. This
spec carves out a thin domain layer (`Authority`, `MultisigUpdate`, `Action`) and a single
`codec` module that is the **only** place Strata crates are touched.

The goal is **isolation**, not feature expansion: the behavior of the e2e test and the
existing unit tests stays the same, but UI / handlers / tests stop depending on Strata
types directly.

## Scope

**Included:**
- New domain enum `Authority` with a single variant: `StrataAdmin`.
- New domain struct `MultisigUpdate { role, add_keys, remove_keys, new_threshold }`.
- New domain enum `Action` with a single variant: `MultisigUpdate(MultisigUpdate)`.
- New `codec` module with `encode(&Action) -> Vec<u8>` and `decode(&[u8]) -> Result<Action, _>`,
  which is the only code path that imports Strata crates.
- `Authority::as_str()` / `Authority::from_str()` for wire/API serialization (keeps the
  `"strata_admin"` string used by the orchestrator HTTP contract).
- Refactor `application::proposals::create_update_action` to accept `authority: Authority`
  and `action: &Action` instead of `&str` + `&str`, encoding via `codec` at the boundary.
- Refactor the domain `Proposal` to carry `authority: Authority` instead of `String` (kept
  alongside the existing `action_hex` which remains the canonical signed form).
- Update unit tests in `proposals.rs` and the e2e test in `e2e_propose_sign.rs` to build
  the action through the new domain + codec, removing direct imports of `strata_*` from
  them.

**Documentation updates (included):**
- Review and update `docs/architecture/overview.md` if the new `domain/` + `infrastructure/action_codec` layering changes the documented module layout or dependency diagram.
- Review and update `docs/architecture/adrs/` — if this introduces a new abstraction boundary (domain vs. Strata-owned types via a single codec crossing point), record it as a new ADR or extend an existing one (e.g., ADR-005 layered architecture, ADR-001 Alpen crate dependencies).
- Review and update `docs/2-discovery/archive/09-functional-analysis.md` if the domain entities (`Authority`, `Action`, `MultisigUpdate`, `Proposal`) introduced here refine or contradict the entities described in §2 and §6.1. (Note: 09 is archived; the canonical domain model now lives in the story-map + ADRs.)
- Only touch docs that are actually affected. Do not create new documentation files unless the change introduces a concept not covered by existing docs.

**NOT included:**
- Other authorities (`AlpenAdmin`, `SeqManager`, `SecurityCouncil`, `PayoutAdmin`).
- Other action variants (`Cancel`, `OperatorSetUpdate`, `SequencerUpdate`,
  `VerifyingKeyUpdate`). These are left as future work; `Authority` and `Action` are
  single-variant enums for now (compiles as closed enums, extensible later without
  breaking callers that match exhaustively).
- Authority × Action matrix enforcement — with one variant each, the invariant
  `action.role == proposal.authority` is trivially satisfied and will be enforced in a
  follow-up when a second action type is introduced.
- Signer set / threshold / `last_seqno` state. Per PRD those live onchain and are the
  backend's responsibility to derive from ASM — not part of the client domain.
- Validation beyond hex/borsh roundtrip (no duplicate key detection, no threshold vs
  keys-count checks — deferred until we add more action types).

## Technical Design

### Domain types (pure, no Strata imports)

```rust
// desktop-app/src-tauri/src/domain/authority.rs
pub enum Authority {
    StrataAdmin,
}

impl Authority {
    pub fn as_str(&self) -> &'static str { /* "strata_admin" */ }
    pub fn from_wire(s: &str) -> Result<Self, AuthorityParseError>;
}

// desktop-app/src-tauri/src/domain/action.rs
pub struct MultisigUpdate {
    pub role: Authority,
    pub add_keys: Vec<CompressedPubKey>,   // newtype over [u8; 33]
    pub remove_keys: Vec<CompressedPubKey>,
    pub new_threshold: NonZero<u8>,
}

pub enum Action {
    MultisigUpdate(MultisigUpdate),
}
```

`CompressedPubKey` is a thin newtype over `[u8; 33]` (no Strata dependency) with
`from_hex` / `to_hex` helpers. It validates length on construction.

### Codec (only module that touches Strata crates)

```rust
// desktop-app/src-tauri/src/infrastructure/action_codec.rs
use strata_asm_params::Role;
use strata_asm_txs_admin::actions::{MultisigAction, UpdateAction};
use strata_asm_txs_admin::actions::updates::multisig::MultisigUpdate as StrataMultisigUpdate;
use strata_crypto::keys::compressed::CompressedPublicKey;
use strata_crypto::threshold_signature::ThresholdConfigUpdate;

pub fn encode(action: &Action) -> Result<Vec<u8>, CodecError>;
pub fn decode(bytes: &[u8]) -> Result<Action, CodecError>;

pub fn encode_hex(action: &Action) -> Result<String, CodecError> { /* hex::encode(encode(..)) */ }
pub fn decode_hex(hex_str: &str) -> Result<Action, CodecError>;
```

Internally maps:
- `Authority::StrataAdmin` ↔ `Role::StrataAdministrator`
- `Action::MultisigUpdate(..)` ↔ `MultisigAction::Update(UpdateAction::Multisig(..))`
- `domain::MultisigUpdate` ↔ `StrataMultisigUpdate` + `ThresholdConfigUpdate`
- `CompressedPubKey` ↔ `CompressedPublicKey`

### Updated `Proposal`

```rust
// desktop-app/src-tauri/src/domain/proposal.rs
pub struct Proposal {
    pub action_id: String,
    pub seq_no: u64,
    pub authority: Authority,     // ← was String
    pub status: String,
    pub action_hex: String,        // canonical signed form (stays raw)
    pub signatures: Vec<ProposalSignature>,
}
```

Deserialization from the orchestrator JSON uses a serde adapter that parses the `authority`
string into `Authority::from_wire`.

### Application layer signature change

```rust
// Before
pub async fn create_update_action(
    client: &dyn OrchestratorClient,
    authority: &str,
    action_hex: &str,
    seq_no: u64,
    signature: &Signature,
) -> Result<Proposal, ProposalError>;

// After
pub async fn create_update_action(
    client: &dyn OrchestratorClient,
    authority: Authority,
    action: &Action,
    seq_no: u64,
    signature: &Signature,
) -> Result<Proposal, ProposalError>;
```

The function encodes `action` via `action_codec::encode_hex` and sends
`authority.as_str()` on the wire. The HTTP contract with the orchestrator is unchanged.

`approve_action` and `get_update_action` are unchanged (they work off `action_id`).

### Flow (unchanged at wire level)

```
caller                domain + codec                 orchestrator_client          HTTP
  │                         │                                │                      │
  │ Action::MultisigUpdate  │                                │                      │
  │ + Authority::StrataAdmin│                                │                      │
  │────────────────────────►│                                │                      │
  │                         │ codec::encode_hex              │                      │
  │                         │ authority.as_str()             │                      │
  │                         │───────────────────────────────►│                      │
  │                         │                                │ CreateProposalRequest│
  │                         │                                │─────────────────────►│
  │                         │                                │                      │
  │                         │                                │◄─────────────────────│
  │                         │ Proposal (authority parsed)    │                      │
  │◄────────────────────────┼────────────────────────────────│                      │
```

### Production code vs. test helpers

**Production:**
- `domain/authority.rs` — `Authority`, `AuthorityParseError`.
- `domain/action.rs` — `Action`, `MultisigUpdate`, `CompressedPubKey`, `PubKeyError`.
- `domain/proposal.rs` — modified: `authority: Authority`.
- `infrastructure/action_codec.rs` — `encode`, `decode`, `encode_hex`, `decode_hex`, `CodecError`.
- `application/proposals.rs` — updated signature of `create_update_action`.

**Test helpers (under `#[cfg(test)]` or `tests/common/`):**
- `demo_multisig_update_action()` — returns a sample `Action::MultisigUpdate(..)` built
  **through domain types**, not Strata. Replaces the current `build_demo_action_hex`.
- `generate_keypair`, `sign_action` — unchanged, live in tests only.

No test helper is exported as a Tauri command or in the public `lib.rs` API.

## Test Cases

All tests target production functions (`codec::encode_hex`, `codec::decode_hex`,
`proposals::create_update_action`, etc.), not the test helpers.

### Unit tests — `infrastructure/action_codec.rs`
1. **Roundtrip**: `decode_hex(encode_hex(action))` returns the same `Action` (structural equality).
2. **Encodes to canonical borsh**: `encode(action)` produces the exact same bytes as the
   direct `borsh::to_vec(&strata MultisigAction)` call — guarantees backward compatibility
   with the signed form used today.
3. **Decode rejects malformed hex**: `decode_hex("zz")` returns `CodecError`.
4. **Decode rejects valid hex but non-MultisigUpdate variant**: a borsh payload of
   `MultisigAction::Update(UpdateAction::Sequencer(..))` returns a
   `CodecError::UnsupportedVariant`.

### Unit tests — `domain/authority.rs`
5. **Wire roundtrip**: `Authority::from_wire(Authority::StrataAdmin.as_str())` returns
   `StrataAdmin`.
6. **Unknown authority**: `Authority::from_wire("unknown")` returns
   `AuthorityParseError::Unknown("unknown".into())`.

### Unit tests — `domain/action.rs`
7. **`CompressedPubKey::from_hex` rejects wrong length**: e.g. 32 bytes → `PubKeyError`.

### Unit tests — `application/proposals.rs` (existing tests, migrated)
8. **`test_create_update_action`**: builds `Action` through the domain, calls
   `create_update_action(&mock, Authority::StrataAdmin, &action, 1, &sig)`, asserts the
   mock received `authority == "strata_admin"` and the expected `action_hex`.
9. **`test_create_then_get_consistent`**: same migration.
10. **`test_signature_is_verifiable`**: same migration.
11. **`test_create_backend_error_propagates`** and **`test_approve_backend_error_propagates`**:
    same migration (errors still propagate through `ProposalError::Orchestrator`).

### E2E test — `e2e-tests/tests/e2e_propose_sign.rs`
12. **`test_e2e_propose_approve_verify`** (existing, migrated): the only `strata_*`-touching
    line (`build_demo_action_hex`) is replaced by a call into `desktop_app::domain` +
    `desktop_app::infrastructure::action_codec::encode_hex`. The test no longer imports
    `strata_asm_params`, `strata_asm_txs_admin`, or `strata_crypto`.

### Error paths explicitly covered
- Malformed hex → `CodecError`
- Unknown authority wire string → `AuthorityParseError`
- Wrong-length pubkey → `PubKeyError`
- Backend HTTP failure → `ProposalError::Orchestrator`

### Not covered (out of scope, documented in spec)
- Authority × Action matrix cross-validation (only one variant each).
- Multi-authority sessions.

## Module structure

```
desktop-app/src-tauri/src/
├── domain/
│   ├── mod.rs
│   ├── authority.rs      ← NEW: Authority enum + wire (de)serialization
│   ├── action.rs         ← NEW: Action, MultisigUpdate, CompressedPubKey
│   ├── proposal.rs       ← MODIFIED: authority: Authority
│   └── session.rs        (unchanged)
├── infrastructure/
│   └── action_codec.rs   ← NEW: ONLY module importing strata_* crates
├── application/
│   ├── mod.rs
│   ├── orchestrator_client.rs  (unchanged)
│   └── proposals.rs            ← MODIFIED: signature + internal encode call
└── lib.rs                ← MODIFIED: re-export domain::{Authority, Action, MultisigUpdate}
                                      and infrastructure::action_codec for e2e-tests
```

**Single-responsibility statement per new module:**
- `domain/authority.rs` — *Represents which multisig role is acting; nothing else.*
- `domain/action.rs` — *Represents the governance actions the client can build, with no
  dependency on Strata crates.*
- `infrastructure/action_codec.rs` — *Translates between domain `Action` and the
  Strata-owned `MultisigAction` borsh form; the single crossing point to Strata crates.*

**Dependency direction check:**
- `application::proposals` depends on `domain::{Authority, Action}` and calls
  `infrastructure::action_codec::encode_hex` — application → domain + infrastructure ✓
- `infrastructure::action_codec` depends on `domain::{Authority, Action, MultisigUpdate}`
  and Strata crates — infrastructure → domain ✓
- `domain::*` depends on nothing (pure types) ✓
- Strata crates appear only inside `infrastructure::action_codec` — contained ✓

**Reusability across crates:**
The e2e test consumes these types via the existing `desktop-app` lib crate. No new crate
is introduced. `lib.rs` re-exports `domain::{Authority, Action, MultisigUpdate,
CompressedPubKey}` and `infrastructure::action_codec` so the e2e test can write:

```rust
use desktop_app::domain::{Action, Authority, MultisigUpdate, CompressedPubKey};
use desktop_app::infrastructure::action_codec;
```
