# Proposal lifecycle states

Requested in [#432](https://github.com/wakeuplabs-io/alpen-multisig/issues/432) so state changes can
be reviewed against expectations.

A proposal carries **two** independent states. `status` is where the proposal sits in its lifecycle;
`broadcastStatus` tracks the commit+reveal bundle on Bitcoin and only matters while `status` is
`approved`.

## `status` — lifecycle

| State | Definition | Set by |
|---|---|---|
| `pending` | Collecting signatures offchain. | Creation. |
| `approved` | Quorum reached; the bundle can now be broadcast. | Backend, once signatures ≥ threshold and the on-chain threshold snapshot is still current. |
| `enacted` | The ASM applied the change. **Not** the same as the reveal being confirmed on Bitcoin. | Backend, after the activation delay. |
| `canceled` | Cancelled during the approved window. | A cancel proposal reaching quorum. |
| `expired` | Never reached quorum in time (7 days), or was overtaken on chain. | Backend, on read. |

## `broadcastStatus` — the commit+reveal bundle

Only meaningful while `status` is `approved`.

| State | Definition | Send button |
|---|---|---|
| `idle` | Nothing sent yet. | **Send** |
| `commit_broadcasted` | Commit tx in the mempool. | hidden |
| `commit_confirmed` | Commit tx mined; reveal goes out next. | hidden |
| `reveal_broadcasted` | Reveal tx in the mempool. | hidden |
| `reveal_confirmed` | Both txs on chain. Nothing left to send — waiting on the ASM. | hidden |
| `failed` | The bundle was not broadcast. | **Retry send** |

## Answering the two questions in #432

**Why was Send still offered after sending?** The detail screen gated the button on quorum alone,
so it ignored `broadcastStatus`. It now follows the table above. The dashboard already did.

**How do I know the bundle is confirmed?** `reveal_confirmed` is the answer, and both screens now
say so in words — *"Reveal confirmed — awaiting ASM enactment"* — instead of showing an
undifferentiated "Approved". The intermediate stages name which leg is in flight.

Note the deliberate gap: `reveal_confirmed` means Bitcoin has the bundle; `enacted` means the ASM
applied the change. The activation delay sits between them, which is why they are separate states.

## Where the rules live

`desktop-app/src/lib/proposal-send-state.ts` is the single source for both screens, covered by
`npm run test:proposal-send-state`. The UI mirrors the backend rather than adding its own rule: the
repository accepts a re-broadcast only from `idle` or `failed` and rejects every other state with a
conflict, which is exactly when the button is offered.
