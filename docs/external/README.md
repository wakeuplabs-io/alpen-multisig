# Strata Multisig — External Documentation

This directory contains client-facing deliverables for the Strata Multisig application. These documents are intended for delivery to Alpen Labs and describe the delivered scope: architecture, API, setup, operations, and quality artifacts for the current release line.

## Document Index

### Getting Started

| Document | Description | PRD Reference |
|----------|-------------|---------------|
| [Setup Guide](./setup-guide.md) | Installation, first-run setup, and configuration | PRD §1.4 |
| [Local Dev Smoke Test Guide](./local-dev-smoke-test-guide.md) | Beginner-friendly, end-to-end walkthrough to run the full app from source and complete a governance action on a local regtest network | PRD §1.4 |
| [Hardware Wallet Compatibility Matrix](./hardware-wallet-matrix.md) | Supported devices and requirements | PRD §3.2 |
| [Verifying What You Sign](./verifying-what-you-sign.md) | What each hardware signer displays, the canonical message format, and how to check the SHA-256 yourself | Issue #402 |

### Architecture and Design

| Document | Description | PRD Reference |
|----------|-------------|---------------|
| [Architecture Overview](./architecture-overview.md) | System design, components, and data flow | Proposal §Technical Approach |
| [API Reference](./api-reference.md) | Backend API endpoints and authentication | Backend PRD §3-§4 |
| [Protocol Research and Integration Assessment](./research-assessment.md) | Protocol integration, update types, and technical assessment | Proposal §Deliverables |
| [Proposal Lifecycle States](./proposal-lifecycle-states.md) | Every proposal and broadcast state, what each means, and when the Send button appears | Issue #432 |

### Build and Release

| Document | Description | PRD Reference |
|----------|-------------|---------------|
| [Build and Release Process](./build-and-release-process.md) | Build pipeline, packaging, and distribution | PRD §1.1, §1.2, §1.3, §1.4 |
| [Release Signing](./release-signing.md) | Cryptographic signing approach and multi-signer support | PRD §1.3 |
| [Verifying Releases](./verifying-releases.md) | Step-by-step verification instructions | PRD §1.3 |
| [Reproducible Builds](./reproducible-builds.md) | Independent build verification | PRD §1.2 |

### Quality and Security

| Document | Description | PRD Reference |
|----------|-------------|---------------|
| [Integration Test Report](./integration-test-report.md) | Test coverage and results | Proposal §Deliverables |
| [Security Review Summary](./security-review-summary.md) | Security analysis and recommendations | Proposal §Deliverables |

## Document Standards

All external documents follow these principles:

1. **PRD Traceability** — Each document references the specific PRD requirement or proposal deliverable it satisfies
2. **Client-Focused** — Written for a technical audience at Alpen Labs
3. **Self-Contained** — Each document can be read independently without requiring access to internal development documentation
4. **Current Scope** — Documentation reflects the current implementation status. Some features (e.g., Alpen Administrator VK update enactment detection, Security Council support) are planned for later phases — see [Architecture Overview](./architecture-overview.md) limitations and open items described there.
5. **No Internal References** — No references to internal development phases, tracking codes, or working documents

## Quick Start

For new users or evaluators:

1. Start with the [Setup Guide](./setup-guide.md) to install and configure the application
2. Review the [Hardware Wallet Compatibility Matrix](./hardware-wallet-matrix.md) to ensure your device is supported
3. Read the [Architecture Overview](./architecture-overview.md) to understand the system design
4. Consult the [API Reference](./api-reference.md) for backend integration details

## Verification

Before using any release:

1. Download the release artifacts and signature files from GitHub Releases
2. Follow the [Verifying Releases](./verifying-releases.md) guide to verify authenticity
3. Optionally, follow the [Reproducible Builds](./reproducible-builds.md) guide to independently verify the build

## Support

For issues, questions, or feedback:

- **GitHub Issues:** [https://github.com/wakeuplabs-io/alpen-multisig/issues](https://github.com/wakeuplabs-io/alpen-multisig/issues)
- **Documentation:** This directory provides documentation for the delivered scope of the system

## Document Maintenance

These documents are maintained alongside the codebase and updated with each release. All changes go through review to ensure accuracy and completeness.
