# Spec: Authority selection filtered by canonical signer set (POC-4)

## Objective

Introduce a minimal authority-selection flow where a signer picks a wallet address and sees
only the multisig authorities where that address belongs to the **canonical signer set**
derived from ASM state. For the walking skeleton, the functional scope is limited to
`Strata Admin`.

The goal is **correct authority scoping and isolation**, not feature expansion: after
selection, the session is bound to exactly one authority and no proposal data from other
authorities is exposed through this flow.

## Scope

**Included:**

- New authority-list command `list_selectable_authorities(signer_pubkey_hex)` returning
  authorities where the signer is eligible.
- Canonical signer-set lookup for `Strata Admin` derived from admin state RPC (no hardcoded
  signer set).
- Authority filter logic at the command boundary: return only selectable authorities.
- Session authority synchronization between React and Tauri:
  - selecting an authority sets backend-side `selected_authority`
  - clearing wallet/session resets backend-side authority scope
- Proposal listing requires both bearer token and selected authority scope.
- Dev-only UI mock mode for authority selection:
  - `VITE_AUTHORITY_SELECTION_MOCK=true` enables mocked authority results
  - optional `VITE_AUTHORITY_SELECTION_MOCK_PROFILE` controls mock dataset
    (`eligible`, `empty`, `mixed`)
- Unit tests for authority filtering helpers in the authority command module.

**Documentation updates (included):**

- Update this spec to reflect the implemented contract (`list_selectable_authorities`) and
  dev mock behavior.

**NOT included:**

- Full runtime support for all 5 authorities in canonical-set derivation.
- Backend session issuance/authentication flow redesign.
- Additional proposal commands beyond `list_proposals`.
- Frontend test framework introduction.

## Technical Design

### Authority listing contract

Tauri exposes:

```rust
#[tauri::command]
async fn list_selectable_authorities(signer_pubkey_hex: String)
    -> Result<Vec<AuthorityEligibility>, String>;
```

For Slice 0, canonical membership is computed for `strata_admin` only:

- if signer is in canonical key set -> returns `[{"authority":"strata_admin","eligible":true}]`
- otherwise returns `[]`

`AuthorityEligibility` remains list-based and extensible for future authorities.

### Session authority scoping

Tauri exposes:

```rust
#[tauri::command]
fn set_selected_authority(authority: Option<String>) -> Result<(), String>;
```

Behavior:

- `Some("strata_admin")` sets scoped authority in Tauri state
- `None` clears scoped authority
- unknown authority string is rejected

`list_proposals` uses:

- bearer token from Tauri state (`session_token`)
- `x-session-authority` from Tauri state (`selected_authority`)

If no authority is selected, proposals listing fails fast.

### UI behavior and mock mode

`AuthoritySelectionScreen` behavior:

- Normal mode:
  - calls `list_selectable_authorities`
  - renders returned list
  - shows empty state when no authority is selectable
- Mock mode (dev only):
  - when `import.meta.env.DEV && VITE_AUTHORITY_SELECTION_MOCK === "true"`
  - skips Tauri membership call
  - uses `VITE_AUTHORITY_SELECTION_MOCK_PROFILE`:
    - `eligible` -> one selectable `strata_admin`
    - `empty` -> no selectable authorities
    - `mixed` -> includes eligible + ineligible rows for UI exercise

On authority selection:

- call `set_selected_authority(authority)` in Tauri
- set React `selectedAuthority`
- navigate to signing screen

On wallet/session clear:

- call `set_selected_authority(null)` in Tauri
- clear React selection

## Test Cases

### Unit tests — `commands/authority.rs`

1. Member path returns selectable `strata_admin`.
2. Non-member path returns empty list.
3. Existing key parsing/extraction tests remain valid.

### Manual verification — UI

4. Normal mode: selectable authority appears only when eligible.
5. Mock mode `eligible`: authority can be selected and navigates.
6. Mock mode `empty`: empty-state message renders.
7. Mock mode `mixed`: ineligible row is disabled.

## Module structure impact

- `desktop-app/src-tauri/src/commands/authority.rs`
  - `list_selectable_authorities`
  - `set_selected_authority`
- `desktop-app/src-tauri/src/state.rs`
  - add `selected_authority` storage
- `desktop-app/src-tauri/src/application/proposals.rs`
  - inject `x-session-authority` header from state
- `desktop-app/src/screens/authority-selection-screen.tsx`
  - list-based rendering + mock toggle paths
- `desktop-app/src/contexts/wallet-session-provider.tsx`
  - clear backend authority scope when wallet/session resets
