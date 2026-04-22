# Alpen Multisig — Non-Functional Items

Requirements extracted from `0-prd/` and `2-discovery/` that do **not** belong in the story map because they carry no direct end-user behavior. These items become specs (architecture + infra + ops) rather than user stories. Do **not** write them as "As a developer, I want…" stories.

Each item is scoped to the concern it addresses; the first round of specs in `docs/specs/` should cover the ones flagged **Slice 0–1** first (they gate the walking skeleton and its early extension).

---

## Distribution, Build & Release

### NF-1 · Cross-platform packaging
- **Concern:** Deployment.
- **Requirement:** The app must run on the latest LTS versions of Debian Linux, macOS, and Windows, on machines with at least 8 GB RAM, 2c4t CPU, 1 TB SSD, 20 Mbps Internet.
- **Source:** UI PRD §1.1.
- **Needed by:** Slice 1.

### NF-2 · Reproducible builds
- **Concern:** Supply-chain security.
- **Requirement:** Application builds must be reproducible per the reproducible-builds.org definition (bit-for-bit output from identical source).
- **Source:** UI PRD §1.2; Proposal §Deliverables.
- **Needed by:** Before first external release.

### NF-3 · Multi-employee signed release binaries
- **Concern:** Supply-chain security.
- **Requirement:** Release binaries must be cryptographically signed by multiple Alpen Labs employees; verification instructions must be published for end users.
- **Source:** UI PRD §1.3; Proposal §Deliverables.
- **Needed by:** Before first external release.

### NF-4 · One-command / one-click install
- **Concern:** UX polish / deployment.
- **Requirement:** Install or launch must be achievable with a single terminal command or double-click; dependency install must take at most one additional command or click.
- **Source:** UI PRD §1.4, §1.4.1.
- **Needed by:** Slice 1.

---

## Backend Coordination Guarantees

### NF-5 · Coordination-only invariant
- **Concern:** Architecture.
- **Requirement:** The backend must not re-define, re-interpret, or enforce any canonical SPS-65 rule (threshold checks, sequence validation, replay protection, update lifecycle, cancellation semantics, confirmation depth). All protocol validity is enforced on-chain.
- **Source:** Backend PRD; Proposal §Technical Approach 3.
- **Needed by:** Slice 0 — foundational invariant for every backend spec.
- **Disposition:** Architectural invariant. Captured in `architecture/` (ADR pending). No backlog item.

### NF-6 · Proposal storage model
- **Concern:** Persistence.
- **Requirement:** Store at minimum three maps: `actions_by_seqno`, `action_by_id`, `sigs_by_id`. Data must be durable and recoverable after restart.
- **Source:** Backend PRD §Data model.
- **Needed by:** Slice 0.
- **Disposition:** Covered as DoD on US-E1. Walking skeleton uses the existing in-memory repository (`orchestrator-be/src/infrastructure/memory_repo.rs`); durable/recoverable storage is deferred beyond the skeleton.

### NF-7 · Idempotent proposal creation
- **Concern:** API contract.
- **Requirement:** `ActionId = hash(MultisigAction, SeqNo)` is stable. Duplicate creation requests for an existing ActionId must be rejected without mutating state.
- **Source:** Backend PRD §ActionId + idempotency.
- **Needed by:** Slice 0.
- **Disposition:** Covered as DoD on US-E1. Already implemented in `memory_repo::save_proposal` (duplicate rejection without mutating state).

### NF-8 · Flexible SeqNo ordering
- **Concern:** Lifecycle.
- **Requirement:** The backend must not enforce strict sequential ordering across proposals. Voluntary metadata-based coordination is acceptable.
- **Source:** Backend PRD §SeqNo handling.
- **Needed by:** Slice 0.
- **Disposition:** Covered as DoD on US-E1.

### NF-9 · Non-authoritative hygiene validation
- **Concern:** API contract.
- **Requirement:** Backend may validate signature shape, duplicate signer indices, and structural integrity. These are hygiene only, never authoritative protocol validation.
- **Source:** Backend PRD §Hygiene checks; Proposal §Technical Approach 3.
- **Needed by:** Slice 0.
- **Disposition:** Deferred — not part of the walking skeleton. To reconsider in a later slice.

### NF-10 · High availability / no single point of failure
- **Concern:** Operations.
- **Requirement:** Backend is expected to operate with high availability and must not be a single point of failure for signer liveness. (Manual fallback — see US-H5 — is the compensating user-facing control.)
- **Source:** Backend PRD §Operations.
- **Needed by:** Before production.

---

## Authentication & Access Control

### NF-11 · Canonical signer set derivation
- **Concern:** Auth.
- **Requirement:** The backend must derive the canonical signer set per authority from the ASM State Transition Function. Signer-set changes on-chain must be reflected in access control.
- **Source:** Backend PRD §Canonical signer set.
- **Needed by:** Slice 0.
- **Disposition:** Covered as DoD on US-C1 and US-C2.

### NF-12 · Authority isolation
- **Concern:** Auth / confidentiality.
- **Requirement:** A signer of one authority is a non-signer for all others. Sessions must be scoped to exactly one authority. No proposal data may leak across authorities.
- **Source:** Backend PRD §Isolation.
- **Needed by:** Slice 0.
- **Disposition:** Covered as DoD on US-C1.

### NF-13 · Bounded session validity
- **Concern:** Auth.
- **Requirement:** Ephemeral sessions must have bounded validity (explicit expiration and/or revocation capability). Sessions must not persist indefinitely.
- **Source:** Backend PRD §Session model; Proposal §Technical Approach 2.
- **Needed by:** Slice 0.
- **Disposition:** Covered as DoD on US-C2.

### NF-14 · Private keys never touch the application layer
- **Concern:** Security invariant.
- **Requirement:** The React frontend never observes private keys, session tokens, or raw sighash bytes. All key-adjacent material is held exclusively in the Tauri Rust process (or the hardware wallet). React sees only session metadata (authority, pubkey, expiry).
- **Source:** `architecture/overview.md`; Proposal §Technical Approach 1.
- **Needed by:** Slice 0.
- **Disposition:** Architectural invariant. Already implicit in the Tauri Rust-backend / React-frontend split (ADR pending). No backlog item.

---

## Hardware Wallet Integration

### NF-15 · Hardware wallet matrix
- **Concern:** Compatibility.
- **Requirement:** Support all HWI-compatible devices that meet all of: Taproot inputs, message signing, on-device display, SPS-65 compatibility. Derivation path fixed at `m/86'/0'/73'/0/n`.
- **Source:** UI PRD §1.6.1, §1.6.2.
- **Discovery note:** Practical matrix narrows to Ledger Nano S+/Stax (`app-bitcoin-new`) and Trezor Model T / Safe 3. BitBox02 partial.
- **Needed by:** Slice 1 (one device) → Slice 2+ (expand).

### NF-16 · SPS-65 digest handling on device
- **Concern:** Protocol compatibility.
- **Requirement:** No consumer device signs a raw 32-byte SPS-65 digest natively. A binding mechanism (synthetic PSBT approach validated in POC-5, or equivalent) is required and must produce signatures that on-chain ASM verification accepts.
- **Source:** `2-discovery/07-hardware-wallet-library-analysis.md`, `2-discovery/16-poc5-trezor-findings.md`; Proposal §Technical Approach 3.
- **Needed by:** Slice 0 — this gates every signing story.
- **Disposition:** Tracked as its own spike/POC item on the sprint board (pending creation). Research + implementation that spans US-F1 and US-H1.

### NF-17 · HWI subprocess bundling
- **Concern:** Deployment.
- **Requirement:** HWI binary (e.g., via PyInstaller) must be bundled and managed as a subprocess by the Tauri Rust backend, on all three target platforms. Windows bundling is the highest-risk platform per discovery.
- **Source:** Proposal §Technical Approach 3; `2-discovery/06-hardware-wallet-architecture.md`.
- **Needed by:** Slice 1.

---

## Observability & Error Handling

### NF-18 · High-signal error messaging
- **Concern:** UX safety.
- **Requirement:** User-facing errors in signing, authentication, and broadcast paths must be explicit and high-signal (no silent failures).
- **Source:** Proposal §Technical Approach (signer safety); general convention in `CLAUDE.md`.
- **Needed by:** Throughout — surfaces in every signing and broadcast story.

### NF-19 · Payload / standardness limits
- **Concern:** Protocol compliance.
- **Requirement:** Broadcast transactions (approvals, cancellations, `block_payout`s) must respect Bitcoin standardness and SPS-51 chunking limits (~395 KB max payload, 520-byte chunks). Automatic `block_payout` selection must not exceed standardness.
- **Source:** UI PRD §1.20.1; `architecture/overview.md` §SPS-51; Proposal §Deliverables.
- **Needed by:** Slice 1 (approvals); Slice 4 (payouts).

---

## Dependencies on Upstream Alpen

### NF-20 · Upstream Alpen crate coverage
- **Concern:** External dependency.
- **Requirement:** The Alpen admin subprotocol crate must expose types/sighash tags for all 13 update types and all 5 authorities. Today only Strata Admin signer update and Sequencer update are covered; 8+ update types and 3 authorities are unsupported.
- **Source:** `2-discovery/08-alpen-crate-prd-coverage.md`.
- **Needed by:** Slice 2 (all update types), Slice 4 (Payout Admin).
- **Action:** Track coordination with Alpen Labs; file upstream issues for each missing piece.

---

## Out of Scope for This Document

- **Functional user stories:** see [`story-map.md`](./story-map.md).
- **Security audit:** explicitly out of scope per `1-proposal/` §Out of Scope. Recommended by WakeUp Labs as a separate engagement before production.
- **Technical design:** specific tech stacks, API schemas, data models — see `architecture/` and `specs/`.
