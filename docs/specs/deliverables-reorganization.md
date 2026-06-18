# Spec: Client-Facing Deliverables Reorganization

> **Execution note (2026-06):** `docs/external/` is populated. Internal duplicates were consolidated:
> `operations/verifying-releases.md` removed (use `external/verifying-releases.md`);
> `operations/reproducible-builds.md`, `release-signing-mvp.md`, and `deliverable/research.md` are
> internal pointers to the matching `external/` documents.

### Completed (2026-06)

| Item | Status |
|------|--------|
| `docs/external/` directory populated | Done — see [`external/README.md`](../external/README.md) |
| Internal duplicate removal / stubs | Done — [#316](https://github.com/wakeuplabs-io/alpen-multisig/pull/316) |
| Stale adversarial assessments | Removed — [#323–#328](https://github.com/wakeuplabs-io/alpen-multisig/pull/323) |
| Internal SSOT navigation map | [`docs/README.md`](../README.md) |

## Objective
Reorganize project documentation to clearly separate client-facing deliverables (external) from internal development artifacts. Ensure all committed deliverables from the proposal and PRDs exist as polished, professional documents suitable for delivery to Alpen Labs.

## Scope

### In scope:
- Audit all existing documents in `docs/` to classify as external or internal
- Identify missing deliverables based on PRDs and proposal commitments
- Propose new directory structure for external deliverables
- Define action plan for each document (move, refactor, create, or leave as-is)
- Establish quality criteria for external documents

### Out of scope:
- Modifying internal development documentation (discovery, architecture, specs, user stories, analysis, features, etc.)
- Implementing new features or code changes
- Modifying PRDs or proposal documents

## Current State Analysis

### Committed Deliverables (from proposal §Deliverables and PRDs)

The proposal explicitly commits to delivering:

1. **Tauri desktop application binary** with reproducible builds
2. **Multi-employee signed release artifacts** with cryptographic verification instructions
3. **Hardware wallet integration** supporting all HWI-compatible devices
4. **Offchain coordination backend** with full update lifecycle
5. **Signing integration layer** consuming Alpen admin subprotocol crate
6. **Complete UI** for all multisig roles
7. **Payout Administrator flow** with manual and automatic block_payout construction
8. **Automated integration test suite** covering all update types
9. **Technical documentation** covering:
   - Architecture overview
   - API reference
   - Build and release process
   - End-user setup guide

PRD-specific requirements:
- PRD §1.2: Builds MUST be reproducible → requires reproducible build documentation
- PRD §1.3: User SHOULD be able to cryptographically verify binary → requires verification instructions
- PRD §1.4: Installation via single command or double-click → requires installation guide

### Existing Documents Classification

#### External (Client-Facing) — Currently Exist

| Document | Location | Status | Action Required |
|----------|----------|--------|-----------------|
| Verifying Releases | `docs/operations/verifying-releases.md` | Good | Move to external, remove internal references, polish for client audience |
| Reproducible Builds | `docs/operations/reproducible-builds.md` | Good | Move to external, remove internal tracking (D4, NF-2), polish for client audience |
| Executable Delivery Plan | `docs/operations/executable-delivery-plan.md` | Mixed | Extract client-facing sections to `build-and-release-process.md`, remove internal tracking |
| Release Signing MVP | `docs/operations/release-signing-mvp.md` | Mixed | Extract client-facing sections to `release-signing.md`, remove "MVP" and internal tracking |
| Research Assessment | `docs/deliverable/research.md` | Good | Move to external as `research-assessment.md`, remove "Phase 1" references |
| Proposal | `docs/1-proposal/01-alpen-multisig-proposal.md` | Good | Keep as-is (already external) |

#### Internal — Do Not Modify

All documents in:
- `docs/2-discovery/` (24 files)
- `docs/3-stories/` (3 files)
- `docs/architecture/` (7 files including ADRs)
- `docs/specs/` (**56** files)
- `docs/archive/features/` (13 directories)
- `docs/analysis/` (1 file)
- `docs/assessment/` (**10** files)
- `docs/evolution/` (13 files)
- `docs/reviews/` (2 files)
- `docs/security/` (1 file)

> **Count note (2026-06):** Spec and assessment counts above were refreshed; re-run `find docs/specs -name '*.md' | wc -l` after large doc changes.

**Hardware wallets (PRD vs implementation):** The proposal commits to HWI-compatible device support. Production uses **Rust-native** Trezor (`trezor-client`) and Ledger (`hwi-rs`) integration — not a bundled HWI subprocess. See [`2-discovery/06-hardware-wallet-architecture.md`](../2-discovery/06-hardware-wallet-architecture.md).

Internal operational documents (remain in `docs/operations/`):
- `runbook.md`
- `desktop-build-linux.md`
- `multi-employee-signing-requirements.md`
- `platform-code-signing-requirements.md`
- `reproducible-builds-research.md`
- `windows-build-incompatibilities.md`
- `windows-portability-upstream-issues.md`

Internal deliverable documents (remain in `docs/deliverable/`):
- `crate-inventory.md`

#### Missing External Deliverables

| Deliverable | PRD/Proposal Reference | Priority | Description |
|-------------|------------------------|----------|-------------|
| Architecture Overview | Proposal §Deliverables | High | Client-facing architecture document covering system design, component boundaries, and technology stack |
| API Reference | Proposal §Deliverables | High | Backend API documentation with endpoints, request/response schemas, authentication flow |
| End-User Setup Guide | Proposal §Deliverables | High | Step-by-step installation and first-use guide for signers |
| Release Signing | Proposal §Deliverables | High | Release signing process and authenticity verification (extracted from release-signing-mvp.md) |
| Build and Release Process | Proposal §Deliverables | High | Build process overview and release artifacts (extracted from executable-delivery-plan.md) |
| Hardware Wallet Compatibility Matrix | Proposal §Deliverables | Medium | List of supported hardware wallets with Taproot/message signing/on-device display capabilities |
| Integration Test Report | Proposal §Deliverables | Medium | Summary of integration test coverage across all update types and multisig roles |
| Security Review Summary | Proposal §Deliverables | Low | High-level summary of security review findings for signing integration and authentication |

## Proposed Structure

```
docs/
├── external/                    # Client-facing deliverables
│   ├── README.md               # Index of external deliverables
│   ├── architecture-overview.md
│   ├── api-reference.md
│   ├── setup-guide.md
│   ├── verifying-releases.md
│   ├── reproducible-builds.md
│   ├── release-signing.md
│   ├── build-and-release-process.md
│   ├── hardware-wallet-matrix.md
│   ├── integration-test-report.md
│   ├── security-review-summary.md
│   └── research-assessment.md  # Protocol research and integration assessment
├── 0-prd/                      # PRDs (do not modify)
├── 1-proposal/                 # Proposal (already external)
├── 2-discovery/                # Internal
├── 3-stories/                  # Internal
├── architecture/               # Internal
├── specs/                      # Internal
├── feature/                    # Internal
├── analysis/                   # Internal
├── assessment/                 # Internal
├── evolution/                  # Internal
├── reviews/                    # Internal
├── security/                   # Internal
├── operations/                 # Internal operational docs
└── deliverable/                # Internal deliverable working docs
```

## Action Plan

### Step 1: Create External Structure
1. Create `docs/external/` directory
2. Create `docs/external/README.md` as index

### Step 2: Move and Refactor Existing Documents

#### High Priority

1. **Verifying Releases**
   - Move `docs/operations/verifying-releases.md` → `docs/external/verifying-releases.md`
   - Refactor: Remove internal references, add PRD §1.3 reference, polish for client audience
   - Ensure self-contained and professional tone
   - Remove any references to internal tracking or phases

2. **Reproducible Builds**
   - Move `docs/operations/reproducible-builds.md` → `docs/external/reproducible-builds.md`
   - Refactor: Remove internal implementation details, add PRD §1.2 reference
   - Focus on "how to verify" rather than "how we implemented"
   - Remove references to "D4", "NF-2", or other internal tracking

3. **Research Assessment**
   - Move `docs/deliverable/research.md` → `docs/external/research-assessment.md`
   - Refactor: Remove internal crate details, focus on deliverables (crate assessment, HW compatibility, architecture)
   - Remove all references to "Phase 1", internal milestones, or development stages
   - Present as a complete technical assessment document

4. **Release Signing (extract)**
   - Keep `docs/operations/release-signing-mvp.md` as internal
   - Extract client-facing sections into new `docs/external/release-signing.md`
   - Include: signing approach, release artifacts, verification flow
   - Remove "MVP" from title, present as complete release signing documentation
   - Remove all references to "D3", "D7", "NF-3", or internal tracking codes

5. **Build and Release Process (extract)**
   - Keep `docs/operations/executable-delivery-plan.md` as internal
   - Extract client-facing sections into new `docs/external/build-and-release-process.md`
   - Include: build process overview, release artifacts, verification flow
   - Remove all references to "D3", "D7", "NF-3", or internal tracking codes

#### Medium Priority

6. **Hardware Wallet Compatibility Matrix**
   - Create `docs/external/hardware-wallet-matrix.md`
   - Extract from `docs/deliverable/research.md` §2 "Hardware Wallet Compatibility Matrix"
   - Expand with device-specific details (Taproot support, message signing, on-device display)
   - Present as a complete reference document without internal references

7. **Integration Test Report**
   - Create `docs/external/integration-test-report.md`
   - Summarize test coverage across all update types and multisig roles
   - Include test environment setup and results summary
   - Present as a complete test coverage report

#### Low Priority

8. **Security Review Summary**
   - Create `docs/external/security-review-summary.md`
   - High-level overview of security review scope and findings
   - Focus on signing integration and authentication flow
   - Present as a complete security assessment

### Step 3: Create New Documents

#### High Priority

1. **Architecture Overview**
   - Create `docs/external/architecture-overview.md`
   - Adapt from `docs/architecture/overview.md` but remove internal implementation details
   - Include: system components, data flow, technology stack, security model
   - Add diagrams (C4 Context and Container level)
   - Present as the complete system architecture

2. **API Reference**
   - Create `docs/external/api-reference.md`
   - Document all backend API endpoints
   - Include: authentication flow, request/response schemas, error codes
   - Reference backend PRD (02-multisig-backend.md) §3-§4
   - Include example requests/responses

3. **End-User Setup Guide**
   - Create `docs/external/setup-guide.md`
   - Step-by-step installation for Linux, macOS, Windows
   - Include: system requirements, installation methods, first-run setup, hardware wallet connection
   - Reference PRD §1.4
   - Include screenshots or diagrams where helpful

### Step 4: Quality Assurance

For each external document, verify:
- [ ] References specific PRD requirement or proposal deliverable it satisfies
- [ ] Written for technical client audience (Alpen Labs engineers)
- [ ] No internal implementation details, development decisions, or working notes
- [ ] No references to MVP, POC, phases, steps, milestones, or internal tracking
- [ ] Self-contained (can be read independently without internal docs)
- [ ] Professional tone and consistent formatting
- [ ] No broken links to internal documents
- [ ] Clear structure with table of contents for longer documents
- [ ] Presented as production-ready, complete documentation

## Quality Criteria for External Documents

### Content Standards

1. **PRD/Proposal Traceability**
   - Every external document must reference the specific PRD requirement or proposal deliverable it satisfies
   - Use format: "Satisfies: PRD §X.Y" or "Deliverable: [description]"

2. **Audience-Appropriate**
   - Written for technical client audience (Alpen Labs engineers and stakeholders)
   - Assume knowledge of Bitcoin, multisig, and hardware wallets
   - Do not assume knowledge of internal development process, tools, or decisions

3. **Self-Contained**
   - Each document can be read independently
   - No references to internal docs (discovery, specs, architecture internals, etc.)
   - If context is needed, include it inline or reference public specifications (SPS-50, SPS-51, SPS-65)

4. **Professional Tone**
   - Clear, concise, technical writing
   - No informal language, internal jargon, or development notes
   - Consistent formatting and structure

5. **Product-Focused Language**
   - NEVER use terms like "MVP", "POC", "prototype", "beta", "experimental", or similar qualifiers
   - This is a production-ready deliverable, not a work-in-progress
   - Avoid references to internal phases, steps, milestones, sprints, or development plans
   - Do not reference internal tracking (e.g., "P-011d", "Phase 1", "Track D", "Wave 2")
   - Present features and capabilities as final, complete, and production-ready
   - Focus on what the product does and how to use it, not how it was developed

### Structure Standards

1. **Document Header**
   - Title
   - Version/date (if applicable)
   - PRD/proposal reference (e.g., "Satisfies: PRD §1.2")
   - Brief description/purpose

2. **Table of Contents**
   - Required for documents > 2 pages
   - Auto-generated if using Markdown

3. **Clear Sections**
   - Use hierarchical headings (H1, H2, H3)
   - Logical flow from overview to details
   - Include diagrams/tables where they aid understanding

4. **References**
   - Link to public specifications (SPS documents, BIPs)
   - Link to other external documents
   - No links to internal docs

## Test Cases

### Document Classification
- Verify each document in `docs/` is classified as external or internal
- Verify internal documents are not modified
- Verify external documents meet quality criteria

### Completeness
- Verify all proposal deliverables have corresponding external documents
- Verify all PRD documentation requirements are satisfied
- Verify no missing deliverables remain after plan execution

### Quality
- Verify each external document references PRD/proposal requirement
- Verify each external document is self-contained
- Verify no internal details leak into external documents
- Verify professional tone and formatting
- Verify no references to MVP, POC, prototype, phases, steps, milestones, or internal tracking codes
- Verify all external documents present the product as production-ready and complete

## Module Structure

### External Documents Directory (`docs/external/`)

**Single responsibility:** Serve as the complete, polished set of client-facing deliverables that evidence PRD compliance and proposal commitments.

**Files:**
- `README.md` — Index and navigation for all external deliverables
- `architecture-overview.md` — System architecture for client audience
- `api-reference.md` — Backend API documentation
- `setup-guide.md` — End-user installation and setup
- `verifying-releases.md` — Release verification instructions
- `reproducible-builds.md` — Reproducible build verification
- `release-signing.md` — Release signing and authenticity verification
- `build-and-release-process.md` — Build process and release artifacts overview
- `hardware-wallet-matrix.md` — Supported hardware wallet devices
- `integration-test-report.md` — Test coverage summary
- `security-review-summary.md` — Security review findings
- `research-assessment.md` — Protocol research and integration assessment

**Internal Documents** (remain unchanged in their current locations)

All development, discovery, architecture, specs, user stories, analysis, features, assessments, reviews, and internal operational docs remain as-is in their current directories.

## Execution Order

1. Create external directory structure
2. Move and refactor existing external documents (verifying-releases, reproducible-builds, research-assessment)
3. Extract client-facing content from mixed documents (executable-delivery-plan, release-signing-mvp)
4. Create high-priority new documents (architecture-overview, api-reference, setup-guide)
5. Create medium-priority documents (hardware-wallet-matrix, integration-test-report)
6. Create low-priority documents (security-review-summary)
7. Create README.md index
8. Quality assurance pass on all external documents — verify no MVP/POC/phase references remain
9. Update AGENTS.md if directory structure changes

## Success Criteria

- All proposal deliverables have corresponding external documents
- All PRD documentation requirements are satisfied
- External documents are polished, professional, and client-ready
- No external document contains references to MVP, POC, prototype, phases, steps, milestones, or internal tracking codes
- Internal documents remain untouched
- Clear separation between external and internal documentation
- External documents can be delivered to Alpen Labs as-is
- All external documents present the product as production-ready and complete
