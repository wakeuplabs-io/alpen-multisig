# **\[External copy\] Strata Multisig Backend \- Design Guidelines & Architectural Notes**

This document provides context and development guidelines for the backend infrastructure supporting the Strata/Alpen administrative multisigs. The user experience is defined in [Strata Multisig UI PRD](https://github.com/alpenlabs/product/blob/john-light-patch-4/strata/prd-strata-multisig-ui.md).

## **1\. Scope**

1. The backend MUST NOT redefine, reinterpret, or override any governance or validity rule defined in [\*\*SPS-65: Strata Administration Subprotocol](https://www.notion.so/265901ba000f80e583d7ff093da6b369?pvs=21).\*\*

2. The backend MUST function exclusively as an offchain coordination service for:

   * Proposal creation.  
   * Signature collection.  
   * Proposal state tracking prior to quorum.  
3. All canonical validity rules, including but not limited to:

   * Signature threshold checks,  
   * Sequence number validation,  
   * Replay protection,  
   * Update lifecycle enforcement,  
   * Cancellation semantics,  
   * Confirmation depth requirements,  
4. MUST be enforced exclusively by the onchain subprotocol implementation.

5. The backend MAY perform basic hygiene checks (e.g., malformed signatures, duplicate signatures, structural validation), but such checks MUST NOT be treated as authoritative protocol validation.

## **2\. Operational Assumptions**

1. The backend is expected to be operated by Alpen Labs and maintained with high availability.  
2. The backend MUST NOT be a single point of failure for the ability of signers to execute valid administrative updates.  
3. In the event that the backend becomes unavailable, signers MUST still be able to:  
   1. Construct valid approval or cancellation transactions  
   2. Aggregate signatures manually,  
   3. Broadcast transactions directly to Bitcoin.

## **3\. Authority Isolation and Access Control**

1. The backend MUST enforce strict separation between multisig authorities.  
2. For a selected multisig authority:  
   1. Only addresses present in the canonical signer set for that authority (as derived from the ASM State) MUST be granted access to:  
      1. View pending proposals,  
      2. Create proposals,  
      3. Submit approval signatures,  
      4. Submit cancellation signatures.  
   2. Any entity whose address is not in the canonical signer set for that authority MUST be treated as a non-signer.  
3. A non-signer MUST NOT be able to view any pending proposals or infer the existence of pending proposals.  
4. A signer of one multisig authority MUST be treated as a non-signer with respect to all other multisig authorities.  
5. Access control decisions MUST be evaluated against the canonical signer set derived from current onchain state.  
6. If the signer set changes onchain:  
   1. The backend MUST update its access control rules accordingly.  
   2. Any session authorization MUST reflect the canonical signer set at the time of authorization.

### **Authentication and Session Model**

1. Every backend request that accesses or modifies multisig state MUST be authenticated.  
2. Authentication MUST provide:  
   1. Proof-of-possession of a canonical signer private key.  
   2. Explicit scoping to a single multisig authority.  
   3. Bounded validity (e.g., expiration or revocation capability).

### **Implementation Notes**

One acceptable authentication mechanism is the use of ephemeral session keys. Under this model:

1. The client generates an ephemeral keypair at session initiation.  
2. The signer signs a structured authentication message using their canonical administrative key. The message MUST:  
   1. Attest to the ephemeral public key.  
   2. Bind the session to a specific multisig authority.  
   3. Include a nonce and/or expiry.  
3. The backend MUST:  
   1. Verify the signature against the canonical signer set derived from the ASM STF.  
   2. Bind the ephemeral public key to the selected authority.  
   3. Treat the ephemeral key as the authenticated session identity.  
4. **All subsequent requests MUST be signed using the ephemeral private key.**

The system includes distinct authority roles (e.g., Strata Administrator, Strata Sequencer Manager, Alpen Administrator). Each role has its own signer set and governance scope as defined onchain.

The backend must enforce strict separation between these roles:

* Signers must only be able to view proposals associated with the multisig(s) for which their address is a canonical signer.  
* Signers must not be able to view proposals belonging to other roles.

The backend must run the ASM STF to get the canonical set of signers for each authority, so that in case of changes to the signing set, the access control is maintained. This is necessary since the execution are delayed.

## **4\. Proposal Semantics**

1. Proposals are identified by:

```rust
ActionId = hash(MultisigAction, SeqNo)
```

2.   
   `SeqNo` MUST be a 64-bit unsigned integer (`u64`).

3. The backend MUST treat `ActionId` as stable and idempotent.

4. If a proposal with the same `(MultisigAction, SeqNo)` already exists:

   1. The backend MUST reject duplicate creation.  
   2. The backend MUST NOT mutate the existing proposal.  
5. The backend MUST support multiple distinct proposals for the same `SeqNo`.

## **4\. Safe Multisig and Deviation**

The administrative multisig model differs from the Safe multisig model on Ethereum.

1. In the Safe Model:  
   1. Proposal `N+1` cannot execute until proposal `N` is executed or explicitly cancelled.  
2. In the Strata/Alpen administrative model:  
   1. A proposal that does not reach quorum MAY be skipped.  
   2. A proposal with a higher `SeqNo` MAY be executed without requiring explicit onchain rejection of earlier unresolved proposals.  
3. The backend MUST NOT enforce strict ordering between sequence numbers.  
4. If signers wish to preserve strict ordering:  
   1. That coordination MUST occur voluntarily.  
   2. The backend MAY expose metadata to support coordination.  
   3. The backend MUST NOT enforce ordering constraints.

# **Code Sketch**

This section sketches the what the backend might look like for a single `Role`.

## **Storage**

At minimum, the backend needs three maps.

```rust
// SeqNo -> Vec<ActionId>
actions_by_seqno: Map<SeqNo, Vec<ActionId>>

// ActionId -> MultisigAction
action_by_id: Map<ActionId, MultisigAction>

// ActionId -> Vec<Signature>
sigs_by_id: Map<ActionId, Vec<Signature>>
```

```rust
/// Minimal backend API for offchain proposal coordination and signature aggregation.
pub trait MultisigBackend {
    type SeqNo;
    type ActionId;
    type Action;
    type Signature;

    /// Return the last confirmed sequence number for this authority.
    /// The canonical source is onchain; the backend may cache.
    fn get_last_seqno(&self) -> Self::SeqNo;

    /// Create a new action and store the creator's signature.
    /// Returns false if the computed ActionId already exists.
    fn create_update_action(
        &mut self,
        action: Self::Action,
        seq: Self::SeqNo,
        sig: Self::Signature,
    ) -> bool;

    /// Append an approval signature for an existing action.
    fn approve_action(&mut self, id: Self::ActionId, sig: Self::Signature);

    /// Fetch the action payload.
    fn get_update_action(&self, id: Self::ActionId) -> Option<Self::Action>;

    /// Fetch signatures collected so far.
    fn get_signatures(&self, id: Self::ActionId) -> Vec<Self::Signature>;

    /// List action ids associated with a particular seqno.
    fn get_action_ids_by_seqno(&self, seq: Self::SeqNo) -> Vec<Self::ActionId>;
}
```

```rust
ActionId = hash(MultisigAction, SeqNo)
type SeqNo = u64;

fn create_update_action(action: MultisigAction, seq: SeqNo, sig: Signature) -> bool {
    let id = compute_action_id(seq, &action);

    // Basic hygiene checks are appropriate server-side (e.g., signature shape,
    // duplicate signer indices). Canonical validity is still enforced onchain.
    validate_sig(id, &sig);

    // Reject duplicates to keep proposal ids stable and idempotent.
    if action_by_id.contains_key(&id) {
        return false;
    }

    actions_by_seqno.entry(seq).or_default().push(id);
    action_by_id.insert(id, action);
    sigs_by_id.entry(id).or_default().push(sig);

    true
}
```

