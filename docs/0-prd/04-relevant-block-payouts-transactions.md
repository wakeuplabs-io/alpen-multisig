```
// https://alpenlabs.notion.site/External-copy-Relevant-block_payouts-transactions-367901ba000f80aa9217c51c8092b1ca
```

# [External copy] Relevant block_payouts transactions

The **Claim** transaction creates the output that is spent by the payout admin in the **Admin Block Payouts** transaction (aka `block_payouts` in the PRD, to be concise).

## Claim Transaction

The operator posts this transaction to start the challenge-response game.

**Inputs:**

- (4 + ω)ε using N/N.

**Outputs:**

- (claim contest connector) (3 + ω)ε locked in tap tree:
  - (Unspendable internal key)
  - For each `i` in `1..=ω` there is a separate tap leaf:
    - N/N and watchtower `i` signature.
  - N/N and relative timelock Δ<sub>Contest</sub>.
- (claim payout connector) ε locked in tap tree:
  - N/N (internal key).
  - Admin signatures (see below).
  - Preimage `unstaking`.
- 0 locked in operator signature (CPFP).

The script for **Admin signatures** looks as follows:

```rust
<admin key 1>
OP_CHECKSIG
<admin key 2>
OP_CHECKSIGADD
<admin key 3>
OP_CHECKSIGADD
# ...
<admin key M> # all admin keys
OP_CHECKSIGADD
<K> # threshold is pushed onto the stack
OP_EQUAL
```

## Admin Block Payouts Transaction

The admin posts this transaction to unilaterally block all payouts of the game.

**Inputs:**

- (claim payout connector) ε using tap leaf:
  - Admin signatures.
- Admin funds (malleable).

**Outputs:**

- Admin change (malleable).

This transaction is malleable, so the number of transaction inputs is unrestricted. In particular, a single Admin Block Payouts Transaction can spend **claim payout connectors** from multiple Claim Transactions at once. To ensure timely propagation, the transaction size should be kept within the standard transaction limit according to the latest Bitcoin Core release.

**Admin signatures** is a script that is defined in the Claim Transaction section above.
