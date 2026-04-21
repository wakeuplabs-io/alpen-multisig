# 12 — Upstream Protocol Readiness — Executive Findings

> **Status:** Complete
> **Audience:** Executive / stakeholder summary
> **Scope:** Consolidated high-level findings from Phase 1 (Protocol Research & Architecture) on the current readiness of the upstream protocol the project depends on.

This note records, at an executive level, the conditions observed in the upstream protocol and its supporting libraries during Phase 1. It is intentionally free of implementation detail; the underlying technical evidence is available in the rest of this folder and in [`docs/deliverable/research.md`](../deliverable/research.md).

## Findings

### 1. Partial functional coverage
The governance scope defined by the product specification is **only partially represented** in the upstream libraries available today. A material portion of the expected authorities and transaction types is not yet present in the public artifacts. Their addition is acknowledged as pending by the provider, without a formally committed delivery date.

### 2. End-to-end transaction lifecycle not yet available
The libraries currently published enable preparatory work such as payload construction and signature computation, but **do not yet provide a complete end-to-end path** to create and broadcast a valid governance transaction to the settlement layer. Components required to close the lifecycle are not part of the public library surface.

### 3. Core protocol artifacts still iterating
Canonical formats, repository location, and foundational type layouts were **modified within the observation window** of this phase. Continued iteration on these foundational artifacts indicates that the protocol has not yet reached a state where stable downstream commitments can be safely anchored to its current interfaces.

### 4. Signing specification under revision
A divergence was identified between the signing format expected by the protocol and the standard capability of common hardware devices used by signers. The provider has acknowledged the gap and committed to aligning the specification; the alignment is **not yet reflected in published code nor in the announced test environment**.

### 5. No mature test environment
There is **no public environment** today that covers the full scope of the protocol. The next announced deployment is known to still be missing recent corrections. No documented path exists for reproducing a complete environment locally, and integration validation is currently only feasible through the provider's internal test suite.

## Overall Maturity Assessment

Taken together, these findings indicate that the upstream protocol — in its current public state — **has not reached the level of maturity required to commit sustained downstream development on it**. Functional scope is incomplete, the end-to-end transaction path is not yet available, foundational artifacts are still evolving, the signing contract is pending revision, and there is no mature environment in which to validate integration behavior.

These observations do not reflect a defect in the upstream effort; they reflect its current stage of development. They do, however, shape the realistic delivery envelope: **investing development time at this stage carries a non-trivial risk of rework** driven by externalities outside the team's control, and any commitment to scope should be calibrated against what is verifiable today rather than what is expected to become available.
