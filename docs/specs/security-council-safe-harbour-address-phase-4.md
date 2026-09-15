# V4 Phase 4 — What the ticket review asked for

> **Functional contract:** [`security-council-safe-harbour-address.md`](./security-council-safe-harbour-address.md)
> — SSOT for *what* V4 must do. This document never overrides it; where it changes the contract, the
> contract is amended in the same pull request.
> **Build plan:** [`security-council-safe-harbour-address-implementation.md`](./security-council-safe-harbour-address-implementation.md).
> **Ticket:** [#547](https://github.com/wakeuplabs-io/alpen-multisig/issues/547) — two review comments.
> **Predecessors:** [Phase 1](./security-council-safe-harbour-address-phase-1.md) (#548),
> [Phase 2](./security-council-safe-harbour-address-phase-2.md) (#549),
> [Phase 3](./security-council-safe-harbour-address-phase-3.md) (#550).
> **Status:** implementation in progress.

## 1. The change in one sentence

The signing message moves from the screen where the proposal is drafted to the screens where it is
signed, and the UI spells the feature the way the product does — *Safe Harbor* — while the ASM's own
bytes keep theirs.

## 2. The signing message is on the wrong screen

The review of #547 read the create form and asked why the *Signing message* section lives there and
not on the page where the signature is given.

The observation is right, and it is sharper than it looks:

- The panel sits in `safe-harbour-address-form-fields.tsx`, a screen at which nothing is signed.
- The screens that *do* sign — the creator's preview (`create-proposal-preview.tsx`) and every
  co-signer's sign view (`sign-proposal-view.tsx`) — render only `DeviceSigningHint`, which returns
  `null` for a software signer (`deviceSigningDisplay` → `none`). So with a software signer the
  message a signer is authorizing appears **only** on a screen where they authorize nothing, and not
  at all for the co-signers, who never see the create form.

### 2.1 The shape

- **Create form:** the panel goes. The form keeps what the contract actually requires of it — see §3.
- **Preview and sign view**, safe harbour rotation only: a *Signing message* section, rendered from
  the same Rust renderer the device signs over (`useDeviceSigningMessage`, already resolved by both
  callers). For a hardware signer `DeviceSigningHint` already prints the message, so the section is
  shown only when the hint is not — the message appears **exactly once** on each signing screen,
  whatever the signer.
- `SigningMessagePanel` moves to `src/components/`, since two domains now render it.

The choice between the hint and the panel is a pure function of the device display, so it is written
as one and tested as a table.

### 2.2 Scope: safe harbour only

Defcon 1 and Defcon 3 keep their panel on the create form. There it is not decoration: the resolved
message is mirrored into the `defconMessage` form value and gates the sign CTA, and both contracts pin
it on that form (V1 AC 4, V2 AC 4). Moving it is a redesign of that gate, not a relocation, and the
review did not ask for it. Recorded as debt in §6.

## 3. The trap: what the create form must still show

Removing the panel naively breaks [Constraint 3](./security-council-safe-harbour-address.md#3-the-reviewable-artifact-is-the-descriptor-hex-not-the-address)
and AC 3, and the ticket's own behaviour list: *next to the address, the form shows the exact value
the signer's device will display*. The **new** destination's descriptor hex appears today only inside
the signing message.

So the form gains that line directly under the input. It is not composed in TypeScript: the action
hex the form already builds (`useSafeHarbourActionHex`) is decoded by the Rust codec
(`decodeActionHex` → `addressHex`), which is the same conversion the device's line comes from.

## 4. Terminology: "Harbor" on screen, "Harbour" on the wire

The second review comment asks every UI string to read *Harbor*, and to leave the raw ASM text alone
while that is raised with the subprotocol's authors.

**Rule:** a string a person reads on screen says *Harbor*. Everything else is unchanged:

- The signing message — `Action: Safe Harbour Address Update`, `New Safe Harbour Address:` — is
  rendered by upstream and is byte-frozen (`roles.rs:42-45`, `updates.rs:76-79`). Changing it would
  invalidate signatures. It stays, and so does its tripwire test.
- Identifiers, file names, `data-testid`s, HTML ids, the wire value `safe_harbour_address_update`,
  code comments and these specs. They are not UI, and renaming them would be a large diff with no
  reader-visible effect that would also drift from the upstream names they mirror.

In scope are the TypeScript copy and the Rust error messages that reach the form
(`domain/action.rs`, `infrastructure/action_codec.rs`).

A signer will therefore see *Safe Harbor* in the app and *Safe Harbour* in the signing message on the
same screen. That is deliberate and temporary; the message must match the device byte for byte.

## 5. Tests

- The hint-or-panel choice as a table: Ledger and Trezor with a resolved message → hint; software
  with a resolved message → panel; nothing resolved → neither.
- The copy tests that pin a sentence (`defcon-copy`, `proposal-send-state`, the Rust address-error
  table) follow the rename in the same commit.
- No DOM tests, as in every prior phase; the walk in §8 is the substitute.

## 6. Debt this phase does not take

- **Defcon's signing message is still on its create form** (§2.2). A follow-up would move it to the
  preview and sign view and re-express the `defconMessage` gate.
- **Mixed spelling on one screen** (§4), until the subprotocol settles its labels.

## 7. Migration

| # | Contents |
|---|---|
| 1 | This spec and the contract amendment. |
| 2 | `SigningMessagePanel` moves to `src/components/`. Pure move. |
| 3 | Safe harbour: the descriptor hex under the create input; the signing message leaves the create form for the preview and sign view. |
| 4 | "Harbour" → "Harbor" in on-screen copy and UI-facing Rust errors, with their tests. |
| 5 | Close-out: the `Status:` headers and the slice board. |

## 8. Verification

The `AGENTS.md` checklist plus `npm run test:unit`. Then, on the local stack, as a Strata
Administrator:

1. Create form: no *Signing message* panel; the new destination's descriptor under the input; the
   menu reads *Safe Harbor address update*.
2. Preview with a software signer: the *Signing message* section with its six lines, still reading
   *Safe Harbour Address Update*.
3. A co-signer's sign view, with a software signer and with a hardware one: the message appears once.
4. With the harbor already active: the notes read *Safe harbor* on create, preview, sign and the
   dashboard.
5. Defcon 1 and 3 unchanged in behaviour, panel still on their create form.

The same walk is the chance to close the three `## Verification` items Phase 3 left open (1, 3, 10).
