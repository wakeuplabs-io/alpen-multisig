# API Reference

**Satisfies: PRD §3 (Backend requirements)** — Offchain coordination service

## Overview

The Orchestrator Backend exposes a versioned HTTP API under `/api/v1` for offchain coordination of multisig proposals. The backend manages proposal creation, signature collection, and lifecycle tracking. It does NOT enforce protocol validity rules — that is the ASM's responsibility.

## Base URL

```
http://localhost:3000/api/v1
```

## Authentication

The API uses an ephemeral-key session model for authentication:

1. The client generates an ephemeral keypair at session initiation
2. The signer signs a structured authentication message using their canonical administrative key
3. The message attests to the ephemeral public key, binds the session to a specific multisig authority, and includes a nonce/expiry
4. The backend verifies the signature against the canonical signer set derived from the ASM state
5. All subsequent requests are signed using the ephemeral private key

**Authentication Flow:**

```
POST /auth/challenge  →  Get challenge nonce
POST /auth/verify     →  Submit signed challenge, receive session token
```

All subsequent requests must include the session token in the `Authorization` header:

```
Authorization: Bearer <session_token>
```

## Endpoints

### Health Check

**GET** `/health`

Liveness probe. Returns 200 if the service is running.

**Response:**
```json
{
  "status": "ok"
}
```

---

### Readiness Check

**GET** `/ready`

Readiness probe. Returns 200 if the service can reach required external dependencies (ASM RPC, Bitcoin RPC).

**Response:**
```json
{
  "status": "ready",
  "asm_rpc": "connected",
  "bitcoin_rpc": "connected"
}
```

**Error Response (503):**
```json
{
  "status": "not_ready",
  "asm_rpc": "disconnected",
  "bitcoin_rpc": "connected"
}
```

---

### List Proposals

**GET** `/proposals`

List proposals, optionally filtered by status. Only returns proposals for the authority associated with the authenticated session.

**Query Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `status` | string | No | Filter by status: `pending`, `approved`, `enacted`, `canceled`, `expired` |

**Response:**
```json
{
  "proposals": [
    {
      "action_id": "a1b2c3d4e5f6...",
      "seq_no": 42,
      "authority": "StrataAdmin",
      "status": "pending",
      "action_hex": "0x...",
      "signatures": [
        {
          "signer_pubkey": "02abc...",
          "signature_hex": "3045..."
        }
      ],
      "quorum_status": {
        "collected": 2,
        "required": 3,
        "is_reached": false
      },
      "created_at": "2026-06-01T12:00:00Z",
      "expires_at": "2026-06-08T12:00:00Z"
    }
  ]
}
```

---

### Create Proposal

**POST** `/proposals`

Create a new proposal with the creator's first signature.

**Request Body:**
```json
{
  "authority": "StrataAdmin",
  "seq_no": 42,
  "action_hex": "0x...",
  "signer_pubkey": "02abc...",
  "signature_hex": "3045..."
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `authority` | string | Yes | Authority role: `AlpenAdmin`, `StrataAdmin`, `StrataSequencerManager`, `SecurityCouncil`, `PayoutAdmin` |
| `seq_no` | integer | Yes | Sequence number (u64) |
| `action_hex` | string | Yes | Hex-encoded serialized `MultisigAction` |
| `signer_pubkey` | string | Yes | Hex-encoded compressed public key of the signer |
| `signature_hex` | string | Yes | Hex-encoded ECDSA signature over the SPS-65 sighash |

**Response (201):**
```json
{
  "action_id": "a1b2c3d4e5f6...",
  "seq_no": 42,
  "authority": "StrataAdmin",
  "status": "pending",
  "action_hex": "0x...",
  "signatures": [
    {
      "signer_pubkey": "02abc...",
      "signature_hex": "3045..."
    }
  ],
  "quorum_status": {
    "collected": 1,
    "required": 3,
    "is_reached": false
  },
  "created_at": "2026-06-01T12:00:00Z",
  "expires_at": "2026-06-08T12:00:00Z"
}
```

**Error Responses:**

| Status | Code | Description |
|--------|------|-------------|
| 400 | `INVALID_HEX` | Invalid hex encoding in action or signature |
| 400 | `MALFORMED_ACTION` | Action payload cannot be decoded |
| 409 | `DUPLICATE_PROPOSAL` | Proposal with same `(seq_no, action_hex)` already exists |
| 409 | `DUPLICATE_SIGNER` | Signer already submitted a signature for this proposal |
| 401 | `UNAUTHORIZED` | Session not authenticated or authority mismatch |

---

### Get Proposal

**GET** `/proposals/:action_id`

Fetch a proposal by its deterministic action ID.

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `action_id` | string | Hex-encoded action ID (SHA-256 hash) |

**Response:**
```json
{
  "action_id": "a1b2c3d4e5f6...",
  "seq_no": 42,
  "authority": "StrataAdmin",
  "status": "pending",
  "action_hex": "0x...",
  "signatures": [
    {
      "signer_pubkey": "02abc...",
      "signature_hex": "3045..."
    }
  ],
  "quorum_status": {
    "collected": 2,
    "required": 3,
    "is_reached": false
  },
  "created_at": "2026-06-01T12:00:00Z",
  "expires_at": "2026-06-08T12:00:00Z"
}
```

**Error Responses:**

| Status | Code | Description |
|--------|------|-------------|
| 404 | `NOT_FOUND` | Proposal with this action ID does not exist |
| 401 | `UNAUTHORIZED` | Session not authenticated or authority mismatch |

---

### Approve Proposal

**POST** `/proposals/:action_id/approve`

Submit an approval signature for an existing proposal.

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `action_id` | string | Hex-encoded action ID |

**Request Body:**
```json
{
  "signer_pubkey": "02abc...",
  "signature_hex": "3045..."
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `signer_pubkey` | string | Yes | Hex-encoded compressed public key of the signer |
| `signature_hex` | string | Yes | Hex-encoded ECDSA signature over the SPS-65 sighash |

**Response:**
```json
{
  "action_id": "a1b2c3d4e5f6...",
  "seq_no": 42,
  "authority": "StrataAdmin",
  "status": "pending",
  "action_hex": "0x...",
  "signatures": [
    {
      "signer_pubkey": "02abc...",
      "signature_hex": "3045..."
    },
    {
      "signer_pubkey": "03def...",
      "signature_hex": "3044..."
    }
  ],
  "quorum_status": {
    "collected": 2,
    "required": 3,
    "is_reached": false
  }
}
```

**Error Responses:**

| Status | Code | Description |
|--------|------|-------------|
| 404 | `NOT_FOUND` | Proposal with this action ID does not exist |
| 409 | `DUPLICATE_SIGNER` | Signer already submitted a signature for this proposal |
| 400 | `INVALID_SIGNATURE` | Signature is malformed or invalid |
| 401 | `UNAUTHORIZED` | Session not authenticated or authority mismatch |

---

### Claim Broadcast Slot

**POST** `/proposals/:action_id/broadcast/claim`

Claim the broadcast coordination slot for a proposal that has reached quorum. This prevents multiple signers from attempting to broadcast simultaneously.

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `action_id` | string | Hex-encoded action ID |

**Response:**
```json
{
  "action_id": "a1b2c3d4e5f6...",
  "broadcast_status": "commit_broadcasted",
  "claimed_by": "02abc...",
  "claimed_at": "2026-06-01T12:30:00Z"
}
```

**Error Responses:**

| Status | Code | Description |
|--------|------|-------------|
| 404 | `NOT_FOUND` | Proposal does not exist |
| 409 | `SLOT_ALREADY_CLAIMED` | Another signer has already claimed the broadcast slot |
| 400 | `QUORUM_NOT_MET` | Proposal has not reached quorum yet |
| 401 | `UNAUTHORIZED` | Session not authenticated or authority mismatch |

---

### Report Broadcast Progress

**PATCH** `/proposals/:action_id/broadcast`

Report broadcast progress and transaction IDs from the desktop client.

**Path Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `action_id` | string | Hex-encoded action ID |

**Request Body:**
```json
{
  "status": "reveal_broadcasted",
  "commit_txid": "abc123...",
  "reveal_txid": "def456..."
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `status` | string | Yes | Broadcast status: `commit_broadcasted`, `reveal_broadcasted`, `confirmed` |
| `commit_txid` | string | Conditional | Bitcoin transaction ID of the commit transaction |
| `reveal_txid` | string | Conditional | Bitcoin transaction ID of the reveal transaction |

**Response:**
```json
{
  "action_id": "a1b2c3d4e5f6...",
  "broadcast_status": "reveal_broadcasted",
  "commit_txid": "abc123...",
  "reveal_txid": "def456..."
}
```

**Error Responses:**

| Status | Code | Description |
|--------|------|-------------|
| 404 | `NOT_FOUND` | Proposal does not exist |
| 401 | `UNAUTHORIZED` | Session not authenticated or authority mismatch |

---

## Data Types

### Authority

| Value | Description |
|-------|-------------|
| `AlpenAdmin` | Alpen Administrator multisig |
| `StrataAdmin` | Strata Administrator multisig |
| `StrataSequencerManager` | Strata Sequencer Manager multisig |
| `SecurityCouncil` | Strata Security Council multisig |
| `PayoutAdmin` | Payout Administrator multisig |

### Proposal Status

| Value | Description |
|-------|-------------|
| `pending` | Proposal created, signatures being collected |
| `approved` | Quorum reached, transaction broadcasted and confirmed on-chain |
| `enacted` | Activation height reached, governance change applied |
| `canceled` | Proposal canceled (off-chain or on-chain) |
| `expired` | 7-day window elapsed before broadcast |

### Action ID

The `action_id` is a deterministic, content-addressed identifier:

```
action_id = SHA256(seq_no_be_bytes ‖ action_hex_bytes)
```

- `seq_no_be_bytes`: 8-byte big-endian representation of the sequence number
- `action_hex_bytes`: Raw bytes of the hex-decoded action payload

This ensures that the same `(seq_no, action)` pair always produces the same ID, providing duplicate rejection and API idempotency.

## Error Handling

All error responses follow a consistent format:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error description"
  }
}
```

**Common Error Codes:**

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `UNAUTHORIZED` | 401 | Missing or invalid authentication |
| `FORBIDDEN` | 403 | Authenticated but not authorized for this action |
| `NOT_FOUND` | 404 | Resource does not exist |
| `INVALID_HEX` | 400 | Invalid hex encoding |
| `MALFORMED_ACTION` | 400 | Action payload cannot be decoded |
| `INVALID_SIGNATURE` | 400 | Signature is malformed or cryptographically invalid |
| `DUPLICATE_PROPOSAL` | 409 | Proposal already exists |
| `DUPLICATE_SIGNER` | 409 | Signer already submitted for this proposal |
| `QUORUM_NOT_MET` | 400 | Proposal has not reached required signature threshold |
| `SLOT_ALREADY_CLAIMED` | 409 | Broadcast slot already claimed by another signer |
| `INTERNAL_ERROR` | 500 | Unexpected server error |

## Rate Limiting

The API does not currently enforce rate limiting. However, clients should implement reasonable request throttling to avoid overwhelming the service.

## Cross-Origin Resource Sharing (CORS)

The API supports CORS for browser-based clients. Allowed origins can be configured via environment variables.

## Related Documents

- [Architecture Overview](./architecture-overview.md) — System design and component boundaries
- [Setup Guide](./setup-guide.md) — Installation and configuration
