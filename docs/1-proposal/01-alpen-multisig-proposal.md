|WakeUp Labs \- Project                                                                                                                       March  2026

# **Alpen Strata Multisig App**

## **Executive Summary**

WakeUp Labs proposes to design and build the Alpen/Strata Multisig Desktop Application, a cross-platform, security-critical tool enabling authorized signers to manage the Strata and Alpen administrative multisigs in a safe, verifiable, and user-friendly environment.

The proposed application consists of three tightly integrated systems: a Tauri-based (Rust \\+ frontend) desktop app, an offchain coordination backend for signature aggregation and update lifecycle management, and a signing integration layer that consumes the existing Alpen protocol crate (admin subprotocol) to construct, sign, and broadcast protocol update messages.

Given the security-critical nature of the application, although no external audit is included, WakeUp Labs will apply the highest rigor to protocol correctness and hardware wallet integration.

## **Technical Approach** 

The application is architected as three distinct but tightly integrated layers. Each layer is scoped independently to allow parallel development tracks and clear ownership boundaries.

### **1\. Desktop Application**

The desktop shell will be built with Tauri, which was selected for its reproducible build support, small binary footprint, native OS integration, and its Rust-based backend, critical for a security-sensitive application. The frontend (React) handles all UI; the Rust core handles cryptographic operations, HWI subprocess management, local node detection, and RPC communication.

Cross-platform targets are Debian Linux (latest LTS), macOS, and Windows. Installation will be achievable via a single command or double-click. Services like [crabnebula](https://docs.crabnebula.dev/cloud/) will be analyzed during the architecture phase. The application defaults to a local Strata node and prompts the user to switch to a remote RPC endpoint if no local node is detected. The RPC endpoint selector will include \`stratabtc.org\` as a built-in trusted preset and support a custom URL field for self-hosted or alternative nodes.

### **2\. Offchain Coordination Backend**

A standalone API server (Axum, Postgres) will manage all offchain state: pending update proposals, partial signatures, quorum tracking, update lifecycle transitions (Pending → Approved → Enacted / Canceled / Expired), and signer authentication via nonce signing.

Access to all pending, canceled, and expired updates is restricted to verified multisig signers. Authentication is based on the Admin ID derived from the user’s connected hardware wallet. The authentication flow uses an ephemeral session key model: at session initiation the client generates an ephemeral keypair; the signer signs a structured message with the Admin ID key attesting to the ephemeral public key, binding the session to the selected multisig authority, and including a nonce and expiry. The backend verifies this signature against the canonical signer set derived from the ASM state transition function (ASM STF), binds the ephemeral key to the authority, and all subsequent API requests are signed with the ephemeral private key. This ensures bounded session validity and explicit authority scoping.

### **3\. Protocol & Signing Layer**

This layer integrates the existing Alpen admin subprotocol crate (crates/asm/subprotocols/admin) to construct, sign, and broadcast the full set of protocol update and approval/cancel messages. We’ll implement the frontend-facing interface that invokes the existing crate logic, wires it to hardware wallet signing, and handles transaction broadcast.

The backend is designed strictly as an offchain coordination layer — it will not enforce canonical validity rules (threshold checks, sequence number validation, replay protection, update lifecycle enforcement, cancellation semantics). All protocol correctness is enforced exclusively by the on-chain subprotocol. The backend may perform basic hygiene checks (malformed signatures, duplicates, structural validation) but these are not authoritative. The application must also function without the backend: signers must be able to construct, aggregate, and broadcast valid transactions manually if the backend is unavailable.

Hardware wallet support will cover all HWI-compatible devices that support Taproot inputs, message signing, and on-device display. Given the absence of a mature single-crate Rust solution covering the full HWI device matrix, the implementation will shell out to a bundled HWI binary (compiled via PyInstaller) managed by the Tauri Rust backend. This approach is the most spec-compliant path. 

The derivation path m/86'/0'/73'/0/n will be used for all signer key derivation. The user selects from the first 20 addresses on this path, can verify the address on-device, and all signing happens on the hardware wallet — private keys never touch the application.

## **Deliverables** 

Key deliverables include:

* Tauri desktop application binary, reproducible builds, installable via single command or double-click on all three target platforms.

* Multi-employee signed release artifacts with documented cryptographic verification instructions for end users.

* Hardware wallet integration supporting all HWI-compatible devices with Taproot \+ message signing \+ on-device display, covering the derivation path m/86'/0'/73'/0/n.

* Offchain coordination backend with full update lifecycle state machine, ephemeral session key authentication, and quorum tracking across all five multisig types.

* Signing integration layer consuming the existing Alpen admin subprotocol crate for all proposal and update types, covering all fifteen or more message types correctly constructed and signable.

* Complete UI for all multisig roles: pending / approved / past update views, proposal creation forms, signature copy/paste flows, raw transaction broadcast, and fee rate controls. Admin Wallet management (balance view, address generation, UTXO visibility, and fee sourcing for administrative transactions)

* Payout Administrator flow: `block_payout` transaction creation (both manual and automatic modes), signing, quorum tracking, broadcast, standardness validation, and historical view.

* Manual `block_payout` construction UI: user-specified inputs, fee rate control (0.1 s/vB increments, up to 10,000 s/vB), Admin Wallet fee sourcing, change-address routing, and Bitcoin Core standardness limit enforcement with critical error messaging.

* Automatic `block_payout` construction UI: "Block payouts" button triggering greedy input selection that maximizes included `block_payout` inputs within standardness limits, accounting for all required signatures and the fee/change structure.

* Payout Administrator flow: block\_payout transaction creation, signing, quorum tracking, broadcast, and historical view.

* Automated integration test suite covering all update types on testnet.

* Technical documentation covering architecture, API reference, build and release process, and end-user setup guide.

## **Timelines and Milestones**

### **Phase 1 — Protocol Research & Architecture (1.5 weeks)**

Goal: Internalize the SPS-50, SPS-51, and SPS-65 specifications; review the existing Alpen admin subprotocol crate and identify integration points; audit hardware wallet device matrix against Taproot \+ message signing requirements; finalize the data model for all five multisig types and fifteen or more update types; define the API contract between backend, signing layer, and frontend; validate HWI bundling approach vs. pure-Rust alternatives.

Must-have deliverables:

* Alpen admin crate integration assessment: API surface, types to consume, any gaps that require extension or workaround.

* Hardware wallet compatibility matrix: devices confirmed to support Taproot inputs, message signing, and on-device display.

* Architecture document: data model, API contract, component boundaries, tech stack confirmation.

* Recommendation on HWI bundling vs. device-narrowing tradeoff, for Alpen Labs sign-off.

**Phase 2 — Product Design & UX Validation (1 week)**

Goal: Translate the architecture outputs and product requirements into a complete product design that defines all primary UX flows. Establish interaction patterns, screen structures, and component behaviors required to support the five multisig types and all update operations. Produce artifacts that enable early stakeholder feedback and reduce ambiguity before frontend implementation begins.

Must-have deliverables:

- UX flows: End-to-end flows for wallet setup, multisig management, transaction proposals, signing, and update operations. Includes flows for both manual and automatic `block_payout` construction modes, standardness error states, and fee/change confirmation screens.  
- Wireframes: Low–mid fidelity screens covering main states and edge cases.  
- Clickable prototype: Prototype for internal feedback and iteration.  
- Frontend reference: Design specs aligned with the API and data model to guide implementation.

### **Phase 3 — Signing Integration & Backend (2.5 weeks)**

Goal: Build the signing integration layer on top of the existing Alpen admin subprotocol crate (all message types, hardware wallet signing flows) and the offchain coordination backend (API, database, auth, state machine, quorum logic). Phases overlap by one week to allow early API integration.

Must-have deliverables:

* Signing integration layer: Consuming the existing SPS-50/51/65 types from the Alpen admin crate for all update types, hardware wallet signing for each role, block\_payout transaction construction, fee rate handling, and signature aggregation logic.

* Backend API: signer authentication, update lifecycle endpoints, signature aggregation, quorum detection, five multisig types fully supported.

* Unit test suite for all protocol message types with testnet validation.

### **Phase 4 — Desktop Application & Frontend (3 weeks)**

Goal: Build the Tauri desktop shell, hardware wallet integration (HWI bundling, address selection, on-device verification), and all UI screens and flows. Phases overlap to allow early frontend integration with the backend API.

Must-have deliverables:

* Tauri app: local node detection, RPC switching, cross-platform packaging.

* Hardware wallet integration: address listing, on-device display, signing flow for all proposal types.

* All UI screens: multisig selection, pending/approved/past views, proposal forms, signature copy/paste, block\_payout flows (including automatic and manual forms and adjustments).

* Reproducible build pipeline with multi-employee signing and cryptographic verification documentation

### **Phase 5 — Integration, Testing & Hardening (2 weeks)**

Goal: End-to-end integration testing across all multisig types, real-device hardware wallet testing, testnet runs, security review of signing and authentication flows, and final documentation.

Must-have deliverables:

* Integration test suite: all update types, all multisig roles, on testnet with at least two hardware wallet device types.

* Security review of signing integration layer and authentication flow.

* Final documentation: architecture, API reference, build/release process, end-user setup guide.

* Signed release binaries for all three target platforms.

| Phase | Duration |
| :---- | :---- |
| 1 — Research & Architecture | 1.5 weeks |
| 2 — UI/UX Design | 1 week |
| 3 — Signing Integration & Backend | 2.5 weeks |
| 4 — Desktop App & Frontend | 3 weeks |
| 5 — Integration & Hardening | 2 weeks |
| **Total (phases in parallel \- see gantt)** | **7.5 weeks (\*)** |

![][image1]

Disclaimers:

* **(\*)** The estimated timeline may vary depending on the level of access to the current Alpen solutions, including their documentation, source code, and operational behavior. Based on current assumptions, the implementation is expected to take between 8 and 11 weeks, depending on the scope of the research phase and the speed at which technical information and clarifications are provided by the Alpen team.   
  * This estimate assumes timely collaboration and access to the relevant technical resources. Delays in obtaining documentation, code access, architectural details, or technical feedback may impact the overall delivery timeline.

* **UI & UX:** Additionally, the scope assumes a simple and functional desktop user interface, focused on usability and operational clarity for users. The estimate does not include advanced UI/UX design, complex visual components, custom design systems, or mobile/tablet interfaces. Any additional UI/UX requirements beyond this scope may require adjustments to the timeline and budget.

## **Team**

The project will be executed by a focused four-person team:

* **Tech Lead (TL)**: Responsible for overall system architecture and cross-component coherence. Coordinates SPS-50/51/65 specification research and Alpen admin crate integration assessment, defines API contracts and component boundaries, oversees the reproducible build pipeline and multi-employee signing process, and serves as the primary point of contact for technical decisions and milestone sign-off.  
* **Senior Blockchain Engineer**: Owns the signing integration layer — consuming the existing Alpen admin subprotocol crate for SPS-50/51/65 message construction, hardware wallet integration (HWI), the Tauri Rust backend, and Bitcoin/Strata RPC connectivity. Also owns the offchain coordination backend: Axum API, Postgres schema, nonce-based signer authentication, quorum state machine, and update lifecycle management.  
* **Senior Full-Stack Engineer**: Owns the desktop frontend — all Tauri UI screens and flows, 13+ proposal and update forms across all five multisig roles, hardware wallet address selection UX, signature copy/paste flows, fee rate controls, and Payout Administrator flows. Also responsible for backend API integration and cross-platform packaging. Implements the manual `block_payout` construction form, the "Block payouts" automatic mode trigger and pre-broadcast confirmation screen, and all associated error states (standardness limit exceeded, insufficient funds).  
* **DevOps Engineer** (part-time): Responsible for reproducible build pipelines across all three target platforms (Linux, macOS, Windows), multi-employee binary signing infrastructure, CI/CD automation, HWI bundling and dependency packaging, and release artifact publication.

## 

## **Out of Scope**

This proposal does not include a security audit of any component. The protocol crate, the backend API, the authentication flow, or the desktop application. Given the security-critical nature of this application, WakeUp Labs strongly recommends that Alpen Labs commission an independent third-party audit prior to production deployment. Audit scope, timeline, and cost would be defined separately and engaged directly between Alpen Labs and a qualified security auditor.

## 

## **Pricing**

[image1]: <data:image/png;base64,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>