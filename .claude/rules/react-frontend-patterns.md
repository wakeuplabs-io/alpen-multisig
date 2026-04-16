---
paths:
  - "desktop-app/src/**/*.{ts,tsx}"
---

# React Frontend Patterns

## Component and State Design

- Use function components and declarative JSX
- Keep components focused and extract reusable logic into hooks (`use*`) for wallet, session, and proposal flows
- Prefer explicit state machines/reducers for lifecycle-heavy screens (Pending, Approved, Past, Expired, Canceled)
- Keep multisig authority context explicit in state to prevent cross-authority data leakage in the UI
- Prefix handlers with `handle` and keep side effects in well-scoped `useEffect` cleanup blocks

## Product Flow Requirements

- Preserve the required navigation flow: wallet connect -> address select -> multisig select -> nonce sign auth -> multisig dashboard
- Always show signer-visible quorum progress (`collected / required`) for pending actions
- Support copy/paste signature workflows with clear validation and failure feedback
- Keep approved/pending/past views distinct and consistent with backend lifecycle semantics
- Surface expiry windows and countdowns where the spec requires time-bounded actions

## Security and UX Constraints

- Never expose private keys or signing internals in UI state, logs, or persisted storage
- Require clear authority labeling on every action form and details view
- Show explicit errors for invalid signature, non-signer access, and authority mismatch cases
- Prefer deterministic, reviewable payload summaries before hardware-wallet signing prompts
- Keep fee inputs and broadcast actions explicit, especially for `block_payout` and send flows

## Integration Practices

- Keep API adapters typed and separate from presentation components
- Normalize server data at boundaries and avoid leaking transport shapes through the component tree
- Add component/integration tests for wallet/auth transitions and authority-scoped visibility
- Prefer accessibility-friendly controls and descriptive action labels for high-stakes operations
