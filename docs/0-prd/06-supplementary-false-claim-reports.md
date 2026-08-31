```
// https://app.notion.com/p/Strata-multisig-app-supplementary-info-3c8901ba000f80839664e0189abc9c4c
```

# Strata multisig app supplementary info

This document provides supplementary information for the Strata multisig app PRD (currently [here](./05-prd-payout-admin-block-payouts-update.md)) that mainly provides further information about **"false claim reports"**, as in PRD §6.4.1.

The following discusses three transactions in the transaction graph: **Claim**, **Contest**, and **Ack**. Contest spends an output of Claim, and Ack spends an output of Contest.

---

## Definitions

**How do we classify a false claim?** A false claim is a claim by a faulty operator.

**How do we classify a faulty operator?** A faulty operator is an operator who posted a claim transaction and later on in the transaction graph, a watchtower was able to post an **Ack** transaction.

Therefore, a **"false claim report"** should refer to two transactions:

- a **Claim** transaction — the transaction the admin needs to block; and
- a **(previous) Ack** transaction.

The claim and the ack should refer to (transaction graphs of) the **same operator**.

---

## Given a Claim transaction, how do we associate it with an operator?

While a claim transaction can be checked against a specific operator, it is **not authenticated** (anyone can publish a transaction with corresponding data on any operator). Therefore we rely on the **Contest** transaction (assuming it exists) to identify the operator.

### Authenticity

A contest transaction includes the **N/N covenant**. To authenticate it (verify that a transaction is indeed a contest transaction), one needs to compare the N/N bridge key in the contest transaction against the known bridge key: parse the leaf script from the contest's input witness and compare the N/N key.

Note: since the transaction appears on-chain, the correctness of the signature against the N/N key was done on Bitcoin.

### Operator

The 1st output of the contest transaction ("**contest proof connector**") has the operator pubkey tweaked with the game index, and so contest captures the operator this graph belongs to and deposit index associated with it.

- One needs to know the **operator public key** and the **deposit index**. These can be checked against a list of possible values, if not provided explicitly.
- The deposit index is not really important for blocking the payout; however it is part of the transaction, so to identify the operator, we also need to know the deposit index.

The reference implementation shows how to check for authenticity and identify the operator.

**Test transaction:** here is a signet Claim transaction where the corresponding operator index is **3** and deposit index is **1**.

---

## Given an Ack transaction, how do we associate it with an operator?

This is quite simple, given the above: from Ack get the contest transaction that it spends, and apply the above.

Note that one also needs to check that the given Ack transaction is indeed in the correct format (because the contest's output that is being spent can also be spent by other transactions): the check must reference the contest txid — an ack spends the **contest payout connector** and does **not** spend the contest proof connector.

Moreover, note that for uniformity, one may supply a claim transaction, and the contest and ack could be derived from it.

The reference implementation shows how, given a claim, we check whether an Ack is posted for the corresponding graph.

**Test transaction:** here is a signet Claim transaction that ends up with Ack (the operator index is **2** and deposit index is **0**).

---

## Config

Some config should be provided with the following (see reference in the Notion source).

Here is an example config, applicable for the test transactions above:

```
network = signet
n_of_n_pubkey = e5b7273af014acd41112d67377be1543499a642e8891481141d578c7df698497
proof_timelock (Δproof) = 24 blocks
game_index = deposit_idx + 1
max_deposit_idx = 99
operator pubkeys (x-only):
  [0] ff79389655916a41e7f8278c1de678ed34c17171122afece179b8a7583a84450
  [1] 19ef09eaecff5c4cc875f9ac56c1849712ed4019d0280ad08c743a8635c796e3
  [2] 7e2b01bdbc6925f103d2157f7494bc4feebc744066042d3618351b29991cfdce
  [3] 35be0db46188725d717ae159ba2b56c971d718b8c827d405664b987390091b90
  [4] 72eb41053d4dfafe53f237cf37071220b54ffa782435dba94a2009063296c565
```

Note: we must check that the "false claim reports" — the ack transaction and the claim transaction that the admin should block — belong to the **same operator**.

---

## Design decisions

1. **Marking a faulty operator once.** We only need to mark an operator as faulty once. We can use memory and remember that an operator is faulty (by checking the existence of Ack and writing down the operator key and index), or we can require that each time we want to block payouts, the user must also provide the Ack transaction (or the corresponding claim; see next point).

2. **What inputs to require?** For the Claim that we want the admin to block, it makes sense to provide the claim transaction. For the Ack we can just require the Ack transaction (and the code will have to fetch the contest transaction to check the operator). For uniformity, we should **always require the Claim transaction (txid)** and under the hood the relevant transactions will be fetched.

3. **Deposit index.** The implementation should have a config that contains an ordered list of operators; however the deposit index is something that should either be supplied or found. In the worst case, the user does not know the deposit index and the implementation must range over possible values (deposit index starts from 0 and increments by 1 with each deposit). It makes sense to let the user the option to supply it exactly, or provide a range (there is a chance the user only knows the possible starting point though).

4. **Dynamic operator list.** Operators may be added or removed, so this should also be taken into account. Subsequently, with each change, the bridge key is also changed. Note that we may have Claim transactions that correspond to either the old list or the new list, so we **cannot just overwrite** the previous values when the operator set changes.

---

## Source note

Received from Alpen Labs on **2026-08-31** (Notion export). Complements [PRD §6.4.1](./05-prd-payout-admin-block-payouts-update.md) (false claim reports and proof validation). For the on-chain shape of the block payout transaction itself, see [Relevant block_payouts transactions](./04-relevant-block-payouts-transactions.md).
