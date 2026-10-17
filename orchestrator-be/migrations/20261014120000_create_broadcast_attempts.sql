-- Every commit + reveal pair sent for a proposal (#605). A signer who did not send the first
-- pair can send their own; the ASM acts on whichever reveal is mined first. The proposal row's
-- commit_txid / reveal_txid stay the pair it tracks now.
CREATE TABLE IF NOT EXISTS broadcast_attempts (
    action_id TEXT NOT NULL REFERENCES proposals(action_id) ON DELETE CASCADE,
    broadcaster_pubkey TEXT,
    commit_txid TEXT NOT NULL,
    reveal_txid TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (action_id, reveal_txid)
);

-- Pairs registered before this table existed. Who sent them was never stored. Txids are kept
-- lowercase, as the application writes them, so the primary key matches a later report of the pair.
INSERT INTO broadcast_attempts (action_id, broadcaster_pubkey, commit_txid, reveal_txid, created_at)
SELECT action_id, NULL, LOWER(commit_txid), LOWER(reveal_txid), COALESCE(broadcast_claimed_at, updated_at)
FROM proposals
WHERE commit_txid IS NOT NULL AND reveal_txid IS NOT NULL;
