# Verifying what you sign

Written for [#402](https://github.com/wakeuplabs-io/alpen-multisig/issues/402) so a signer can
confirm, without trusting the application, that the message their hardware signer is about to sign is
the governance action they intended.

> **Pending review.** SPS-65 has no public specification to reference, so the message format below is
> documented from the ASM source at the revision this release pins
> (`e0461f8f520e9be814541d1f76fb961fd847e4ae`). Confirm it before treating it as canonical.

## The short version

Your hardware signer shows you the message it is signing. The app shows you the same thing. **Compare
them.** If they differ, reject on the device — the signature is only meaningful if what the device
signed is what you read.

The app no longer shows the SPS-65 sighash. It is a real value used by the protocol, but **no
hardware signer ever displays it**, so it could not be compared against anything and only invited
false confidence.

## What each device shows

| Signer | On screen | What the app shows |
|---|---|---|
| Trezor | The message text. | The same message text. |
| Ledger | Either the message text **or** a `Message hash`. | Both, so one of them always matches. |
| Software wallet | Nothing — there is no device screen. | Nothing to compare; the host machine is trusted. |

A Ledger running Bitcoin app **2.2.2 or later** renders the message text when it is printable and
**640 bytes or shorter**, and the `Message hash` otherwise. In practice a signer-set change crosses
640 bytes at about six added or removed members. **Before 2.2.2 the device always showed the hash**,
whatever the message — and the app cannot tell which Bitcoin app version your device is running.
That is why it shows both values and asks you to match whichever one appears.

The `Message hash` is `SHA-256` of the message text, printed by the device in **upper case**. The app
prints it upper case too, so the two strings are identical character for character.

## The message format

```
Strata ASM Administration v1
Action: <action type>
Authorized By: <role>
Sequence: <number>
Action Details:
  <detail line>
  <detail line>
```

Example of a signer-set change:

```
Strata ASM Administration v1
Action: Alpen Administrator Multisig Update
Authorized By: Alpen Administrator
Sequence: 7
Action Details:
  New Threshold: 2
  Members to Add: 1
  1. Add Member: 0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798
  Members to Remove: 0
```

What to check before approving:

- **`Action`** names the change you asked for. The full vocabulary is `Cancel`, `Strata
  Administrator Multisig Update`, `Strata Sequencer Manager Multisig Update`, `Alpen Administrator
  Multisig Update`, `Bridge Operator Set Update`, `Sequencer Update`, `OL STF VK Update`, `ASM STF VK
  Update`, `EE STF VK Update`. Anything else is not a message this protocol produces.
- **`Authorized By`** is one of `Strata Administrator`, `Strata Sequencer Manager`, `Alpen
  Administrator`. You do not need to cross-check it against the action: the role is *derived* from
  the action type, so a mismatched pair cannot be constructed.
- **`Sequence`** matches the proposal you are signing. The number is inside the signed message, so a
  signature for sequence 7 cannot be replayed as sequence 8. Note that this procedure cannot tell you
  which sequence number is the *current* one — for that you need a source other than the machine
  running the app.
- Every key under `Action Details` is one you meant to add or remove. **Read the whole key**, not the
  first and last few characters — that is exactly what an attacker substituting a key would rely on.
- Counts are always printed, including `Members to Remove: 0`. A count that does not match the list
  below it means something is wrong.

These labels are byte-stable by design: the ASM source notes that changing them would invalidate
already-signed messages, precisely because external signers hash the rendered text.

### Detail lines by action type

Key lengths differ by action, and a correct message will not always show 66-character keys:

| Action | Detail lines | Value format |
|---|---|---|
| Multisig updates (Strata Admin, Sequencer Manager, Alpen Admin) | `New Threshold`, `Members to Add`, `N. Add Member`, `Members to Remove`, `N. Remove Member` | Lower-case hex, 33-byte compressed key — **66 characters** |
| `Bridge Operator Set Update` | `Operators to Add`, `N. Add Operator`, `Operators to Remove`, `N. Remove Operator` | Lower-case hex, 32-byte x-only key — **64 characters** |
| `Sequencer Update` | `New Sequencer Key` | Lower-case hex, 32-byte — **64 characters** |
| VK updates (`OL STF`, `ASM STF`, `EE STF`) | `Predicate Type`, then `Predicate Hex` **or** `Predicate Hash` | See below |
| `Cancel` | `Target Id`, `Target Update`, then the cancelled update's own detail lines | See below |

**VK updates.** The second line is `Predicate Hex: <hex>` when the condition is 32 bytes or shorter,
and `Predicate Hash: <hex>` when it is longer. Seeing `Predicate Hash` instead of `Predicate Hex` is
normal for a large condition, not a sign of tampering. An unrecognised predicate type renders as
`unknown (<id>)`.

**Cancel.** A cancel message nests the details of the update it cancels:

```
Strata ASM Administration v1
Action: Cancel
Authorized By: Strata Sequencer Manager
Sequence: 3
Action Details:
  Target Id: 5
  Target Update: Sequencer Update
  New Sequencer Key: 79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798
```

So under `Action: Cancel` you will see detail lines belonging to a different action type. That is
expected — check that `Target Update` and the lines under it describe the proposal you mean to
cancel.

## Verifying the hash yourself

If your Ledger shows a `Message hash` and you want to confirm it independently of this application,
compute the SHA-256 of the message text with software you trust.

Copy the text from the app's **Message** box into a file, then:

```bash
tr -d '\r' < message.txt | printf '%s' "$(cat)" | sha256sum
```

The result must equal the hash on the device screen, ignoring case (`sha256sum` prints lower case,
the device prints upper case).

Three details decide whether this works:

- **`$(cat …)` strips the trailing newline** your editor adds. The message is hashed without one.
- **`tr -d '\r'` removes carriage returns.** If your editor saves Windows line endings, every line
  gains a byte the message does not have and the hash will not match anything.
- **Do not let your editor trim trailing whitespace or add a byte-order mark.** Some messages
  legitimately end in a space — a VK update with an empty condition ends with `Predicate Hex: ` —
  and both edits change the hash.

### Byte-level rules, if you are scripting this

| Property | Value |
|---|---|
| Line separator | `\n` (LF) only — never CRLF |
| Trailing newline | None |
| Encoding | UTF-8; in practice ASCII |
| Indent on detail lines | Exactly two spaces |
| List numbering | 1-based, e.g. `  1. Add Member: <hex>` |
| Counts | Always printed, including when zero |
| Hash | A single `SHA-256` over the raw message bytes — **no** `"Bitcoin Signed Message:"` prefix and **no** length prefix |

That last row is the difference between the `Message hash` and the SPS-65 sighash. The signature
itself is a standard Bitcoin signed message, whose digest is a double SHA-256 over the message with
the Bitcoin magic prefix — that digest is the sighash, and it is what the protocol verifies. Your
device computes it internally; it never puts it on screen.

## If the values do not match

**Reject on the device.** Do not approve and then investigate. A mismatch means the message the
device is about to sign is not the one the application showed you, which is what a compromised host
machine would look like. Report it before signing anything else from that machine.
