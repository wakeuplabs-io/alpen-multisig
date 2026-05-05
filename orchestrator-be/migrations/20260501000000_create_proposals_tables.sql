CREATE TABLE IF NOT EXISTS proposals (
    action_id TEXT PRIMARY KEY,
    seq_no BIGINT NOT NULL CHECK (seq_no >= 0),
    authority TEXT NOT NULL,
    status TEXT NOT NULL,
    action_hex TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS proposal_signatures (
    action_id TEXT NOT NULL REFERENCES proposals(action_id) ON DELETE CASCADE,
    signer_pubkey TEXT NOT NULL,
    signature_hex TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (action_id, signer_pubkey)
);

CREATE INDEX IF NOT EXISTS idx_proposals_authority_status ON proposals(authority, status);
CREATE INDEX IF NOT EXISTS idx_proposals_status ON proposals(status);
